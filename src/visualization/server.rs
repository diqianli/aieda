//! Web server for CPU visualization.
//!
//! Provides HTTP and WebSocket endpoints for real-time visualization.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State, Query,
    },
    response::{Html, IntoResponse, Response, Json},
    routing::{get, get_service},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tower_http::{
    services::ServeDir,
    cors::CorsLayer,
};

use super::snapshot::VisualizationSnapshot;
use super::konata_format::KonataSnapshot;
use super::VisualizationConfig;

/// Shared state for the visualization server.
#[derive(Clone)]
pub struct ServerState {
    /// Configuration
    config: VisualizationConfig,
    /// Current snapshots
    snapshots: Arc<RwLock<Vec<VisualizationSnapshot>>>,
    /// Konata snapshots for pipeline visualization
    konata_snapshots: Arc<RwLock<Vec<KonataSnapshot>>>,
    /// Channel for broadcasting snapshot updates
    snapshot_tx: broadcast::Sender<VisualizationSnapshot>,
    /// Channel for broadcasting Konata updates
    konata_tx: broadcast::Sender<KonataSnapshot>,
    /// Current playback state
    playback: Arc<RwLock<PlaybackState>>,
}

impl ServerState {
    /// Add a snapshot to the server state.
    pub async fn add_snapshot(&self, snapshot: VisualizationSnapshot) {
        // Update playback state with total cycles
        {
            let mut playback = self.playback.write().await;
            playback.total_cycles = snapshot.cycle + 1;
        }

        // Store snapshot
        {
            let mut snapshots = self.snapshots.write().await;
            if snapshots.len() >= self.config.max_snapshots {
                snapshots.remove(0);
            }
            snapshots.push(snapshot.clone());
        }

        // Broadcast to connected clients
        let _ = self.snapshot_tx.send(snapshot);
    }

    /// Add a Konata snapshot to the server state.
    pub async fn add_konata_snapshot(&self, snapshot: KonataSnapshot) {
        // Store Konata snapshot
        {
            let mut snapshots = self.konata_snapshots.write().await;
            if snapshots.len() >= self.config.max_snapshots {
                snapshots.remove(0);
            }
            snapshots.push(snapshot.clone());
        }

        // Broadcast to connected clients
        let _ = self.konata_tx.send(snapshot);
    }
}

/// Playback state for animation control.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PlaybackState {
    /// Whether currently playing
    pub is_playing: bool,
    /// Current cycle being displayed
    pub current_cycle: u64,
    /// Animation speed (cycles per second)
    pub speed: u32,
    /// Total cycles available
    pub total_cycles: u64,
}

/// Visualization server.
pub struct VisualizationServer {
    state: ServerState,
    config: VisualizationConfig,
}

impl VisualizationServer {
    /// Create a new visualization server.
    pub fn new(config: VisualizationConfig) -> Self {
        let (snapshot_tx, _) = broadcast::channel(1000);
        let (konata_tx, _) = broadcast::channel(1000);

        let state = ServerState {
            config: config.clone(),
            snapshots: Arc::new(RwLock::new(Vec::new())),
            konata_snapshots: Arc::new(RwLock::new(Vec::new())),
            snapshot_tx,
            konata_tx,
            playback: Arc::new(RwLock::new(PlaybackState {
                is_playing: false,
                current_cycle: 0,
                speed: config.animation_speed,
                total_cycles: 0,
            })),
        };

        Self { state, config }
    }

    /// Get a clone of the server state for external use.
    pub fn state(&self) -> ServerState {
        self.state.clone()
    }

    /// Add a snapshot to the server.
    pub async fn add_snapshot(&self, snapshot: VisualizationSnapshot) {
        // Update playback state with total cycles
        {
            let mut playback = self.state.playback.write().await;
            playback.total_cycles = snapshot.cycle + 1;
        }

        // Store snapshot
        {
            let mut snapshots = self.state.snapshots.write().await;
            if snapshots.len() >= self.state.config.max_snapshots {
                snapshots.remove(0);
            }
            snapshots.push(snapshot.clone());
        }

        // Broadcast to connected clients
        let _ = self.state.snapshot_tx.send(snapshot);
    }

    /// Add a Konata snapshot to the server.
    pub async fn add_konata_snapshot(&self, snapshot: KonataSnapshot) {
        self.state.add_konata_snapshot(snapshot).await;
    }

    /// Run the visualization server.
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], self.config.port));

        // Build router
        let app = Router::new()
            .route("/", get(index_handler))
            .route("/ws", get(websocket_handler))
            .route("/api/snapshots", get(get_snapshots))
            .route("/api/snapshot/:cycle", get(get_snapshot_by_cycle))
            .route("/api/playback", get(get_playback).post(set_playback))
            .route("/api/control", get(control_handler))
            .route("/api/konata", get(get_konata_snapshots))
            .route("/api/konata/:cycle", get(get_konata_snapshot_by_cycle))
            .route("/api/export/konata", get(export_konata))
            .fallback_service(get_service(ServeDir::new("visualization/static")).handle_error(
                |error| async move {
                    (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unhandled internal error: {}", error),
                    )
                },
            ))
            .layer(CorsLayer::permissive())
            .with_state(self.state);

        tracing::info!("Visualization server listening on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }

    /// Run the server in a background task.
    pub fn run_in_background(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            if let Err(e) = self.run().await {
                tracing::error!("Visualization server error: {}", e);
            }
        })
    }
}

/// Index page handler.
async fn index_handler() -> Html<&'static str> {
    Html(include_str!("../../visualization/static/index.html"))
}

/// WebSocket upgrade handler.
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ServerState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

/// Handle WebSocket connection.
async fn handle_websocket(socket: WebSocket, state: ServerState) {
    use futures_util::{SinkExt, StreamExt};

    let (mut tx, mut rx) = socket.split();

    // Subscribe to snapshot updates
    let mut snapshot_rx = state.snapshot_tx.subscribe();

    tracing::info!("WebSocket client connected");

    // Clone state for the receive task
    let recv_state = state.clone();

    // Handle incoming messages and outgoing snapshots
    loop {
        tokio::select! {
            // Receive snapshots and send to client
            result = snapshot_rx.recv() => {
                match result {
                    Ok(snapshot) => {
                        let json = match serde_json::to_string(&snapshot) {
                            Ok(j) => j,
                            Err(e) => {
                                tracing::error!("Failed to serialize snapshot: {}", e);
                                continue;
                            }
                        };

                        if tx.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            // Receive messages from client
            msg = rx.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // Handle control messages
                        if let Ok(control) = serde_json::from_str::<ControlMessage>(&text) {
                            handle_control_message(&recv_state, control).await;
                        }
                    }
                    Some(Ok(Message::Close(_))) => break,
                    Some(Err(_)) => break,
                    None => break,
                    _ => {}
                }
            }
        }
    }

    tracing::info!("WebSocket client disconnected");
}

/// Control message from client.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ControlMessage {
    #[serde(rename = "play")]
    Play,
    #[serde(rename = "pause")]
    Pause,
    #[serde(rename = "step")]
    Step,
    #[serde(rename = "reset")]
    Reset,
    #[serde(rename = "speed")]
    Speed { value: u32 },
    #[serde(rename = "goto")]
    GoTo { cycle: u64 },
}

/// Handle control messages.
async fn handle_control_message(state: &ServerState, control: ControlMessage) {
    let mut playback = state.playback.write().await;

    match control {
        ControlMessage::Play => {
            playback.is_playing = true;
        }
        ControlMessage::Pause => {
            playback.is_playing = false;
        }
        ControlMessage::Step => {
            if playback.current_cycle < playback.total_cycles - 1 {
                playback.current_cycle += 1;
            }
        }
        ControlMessage::Reset => {
            playback.current_cycle = 0;
            playback.is_playing = false;
        }
        ControlMessage::Speed { value } => {
            playback.speed = value.clamp(1, 100);
        }
        ControlMessage::GoTo { cycle } => {
            playback.current_cycle = cycle.min(playback.total_cycles.saturating_sub(1));
        }
    }
}

/// Query parameters for snapshot requests.
#[derive(Debug, Deserialize)]
struct SnapshotQuery {
    /// Start cycle (optional)
    start: Option<u64>,
    /// End cycle (optional)
    end: Option<u64>,
    /// Limit number of results
    limit: Option<usize>,
}

/// Get snapshots handler.
async fn get_snapshots(
    State(state): State<ServerState>,
    Query(query): Query<SnapshotQuery>,
) -> impl IntoResponse {
    let snapshots = state.snapshots.read().await;

    let filtered: Vec<_> = snapshots
        .iter()
        .filter(|s| {
            if let Some(start) = query.start {
                if s.cycle < start {
                    return false;
                }
            }
            if let Some(end) = query.end {
                if s.cycle > end {
                    return false;
                }
            }
            true
        })
        .take(query.limit.unwrap_or(1000))
        .cloned()
        .collect();

    axum::Json(filtered)
}

/// Get a specific snapshot by cycle.
async fn get_snapshot_by_cycle(
    State(state): State<ServerState>,
    axum::extract::Path(cycle): axum::extract::Path<u64>,
) -> impl IntoResponse {
    let snapshots = state.snapshots.read().await;

    match snapshots.iter().find(|s| s.cycle == cycle) {
        Some(snapshot) => axum::Json(Some(snapshot.clone())),
        None => axum::Json(None),
    }
}

/// Get playback state.
async fn get_playback(State(state): State<ServerState>) -> impl IntoResponse {
    let playback = state.playback.read().await;
    axum::Json(playback.clone())
}

/// Set playback state.
async fn set_playback(
    State(state): State<ServerState>,
    axum::Json(playback): axum::Json<PlaybackState>,
) -> impl IntoResponse {
    let mut current = state.playback.write().await;
    *current = playback;
    axum::Json(current.clone())
}

/// Control handler for simple GET-based control.
async fn control_handler(
    State(state): State<ServerState>,
    Query(params): Query<ControlParams>,
) -> impl IntoResponse {
    let mut playback = state.playback.write().await;

    if let Some(action) = params.action {
        match action.as_str() {
            "play" => playback.is_playing = true,
            "pause" => playback.is_playing = false,
            "step" => {
                if playback.current_cycle < playback.total_cycles - 1 {
                    playback.current_cycle += 1;
                }
            }
            "reset" => {
                playback.current_cycle = 0;
                playback.is_playing = false;
            }
            _ => {}
        }
    }

    if let Some(speed) = params.speed {
        playback.speed = speed.clamp(1, 100);
    }

    if let Some(cycle) = params.goto {
        playback.current_cycle = cycle.min(playback.total_cycles.saturating_sub(1));
    }

    axum::Json(playback.clone())
}

/// Control parameters for GET-based control.
#[derive(Debug, Deserialize)]
struct ControlParams {
    action: Option<String>,
    speed: Option<u32>,
    goto: Option<u64>,
}

/// Get all Konata snapshots.
async fn get_konata_snapshots(
    State(state): State<ServerState>,
    Query(query): Query<SnapshotQuery>,
) -> impl IntoResponse {
    let snapshots = state.konata_snapshots.read().await;

    let filtered: Vec<_> = snapshots
        .iter()
        .filter(|s| {
            if let Some(start) = query.start {
                if s.cycle < start {
                    return false;
                }
            }
            if let Some(end) = query.end {
                if s.cycle > end {
                    return false;
                }
            }
            true
        })
        .take(query.limit.unwrap_or(1000))
        .cloned()
        .collect();

    Json(filtered)
}

/// Get a specific Konata snapshot by cycle.
async fn get_konata_snapshot_by_cycle(
    State(state): State<ServerState>,
    axum::extract::Path(cycle): axum::extract::Path<u64>,
) -> impl IntoResponse {
    let snapshots = state.konata_snapshots.read().await;

    match snapshots.iter().find(|s| s.cycle == cycle) {
        Some(snapshot) => Json(Some(snapshot.clone())),
        None => Json(None),
    }
}

/// Export all Konata data as JSON for download.
async fn export_konata(
    State(state): State<ServerState>,
) -> impl IntoResponse {
    let snapshots = state.konata_snapshots.read().await;

    // Combine all snapshots into a single export format
    let export = KonataExport {
        version: "1.0".to_string(),
        exported_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        snapshots: snapshots.clone(),
    };

    Json(export)
}

/// Export format for Konata data.
#[derive(Debug, Serialize)]
struct KonataExport {
    version: String,
    exported_at: u64,
    snapshots: Vec<KonataSnapshot>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let config = VisualizationConfig::enabled();
        let server = VisualizationServer::new(config);
        assert_eq!(server.config.port, 3000);
    }

    #[tokio::test]
    async fn test_add_snapshot() {
        let config = VisualizationConfig::default();
        let server = VisualizationServer::new(config);

        let snapshot = VisualizationSnapshot {
            cycle: 0,
            committed_count: 0,
            instructions: vec![],
            dependencies: vec![],
            metrics: Default::default(),
            pipeline: Default::default(),
        };

        server.add_snapshot(snapshot).await;

        let snapshots = server.state.snapshots.read().await;
        assert_eq!(snapshots.len(), 1);
    }
}
