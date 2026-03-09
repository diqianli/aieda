// ARM CPU Emulator Visualization Client

class CPUVisualization {
    constructor() {
        // WebSocket connection
        this.ws = null;
        this.connected = false;

        // State
        this.snapshots = [];
        this.currentCycle = 0;
        this.isPlaying = false;
        this.speed = 10; // cycles per second
        this.animationFrameId = null;
        this.lastUpdateTime = 0;

        // IPC history for chart
        this.ipcHistory = [];
        this.maxIpcHistory = 100;

        // Konata pipeline view
        this.pipelineView = null;

        // DOM elements
        this.elements = {
            connectionStatus: document.getElementById('connectionStatus'),
            cycleValue: document.getElementById('cycleValue'),
            committedValue: document.getElementById('committedValue'),
            ipcValue: document.getElementById('ipcValue'),
            l1HitValue: document.getElementById('l1HitValue'),
            l2HitValue: document.getElementById('l2HitValue'),
            windowValue: document.getElementById('windowValue'),
            instructionBody: document.getElementById('instructionBody'),
            dependencyGraph: document.getElementById('dependencyGraph'),
            ipcChart: document.getElementById('ipcChart'),
            l1Bar: document.getElementById('l1Bar'),
            l2Bar: document.getElementById('l2Bar'),
            missBar: document.getElementById('missBar'),
            l1Percent: document.getElementById('l1Percent'),
            l2Percent: document.getElementById('l2Percent'),
            missPercent: document.getElementById('missPercent'),
            playBtn: document.getElementById('playBtn'),
            pauseBtn: document.getElementById('pauseBtn'),
            stepBtn: document.getElementById('stepBtn'),
            resetBtn: document.getElementById('resetBtn'),
            speedSlider: document.getElementById('speedSlider'),
            speedValue: document.getElementById('speedValue'),
            stageFetch: document.getElementById('stage-fetch'),
            stageDispatch: document.getElementById('stage-dispatch'),
            stageExecute: document.getElementById('stage-execute'),
            stageMemory: document.getElementById('stage-memory'),
            stageCommit: document.getElementById('stage-commit'),
        };

        // IPC Chart context
        this.ipcCtx = this.elements.ipcChart.getContext('2d');

        // Initialize
        this.initEventListeners();
        this.initTabs();  // This will lazy-initialize the PipelineView when the tab is clicked
        this.loadInitialData();
        this.connect();
        this.startAnimation();
    }

    initEventListeners() {
        this.elements.playBtn.addEventListener('click', () => this.play());
        this.elements.pauseBtn.addEventListener('click', () => this.pause());
        this.elements.stepBtn.addEventListener('click', () => this.step());
        this.elements.resetBtn.addEventListener('click', () => this.reset());
        this.elements.speedSlider.addEventListener('input', (e) => {
            this.speed = parseInt(e.target.value);
            this.elements.speedValue.textContent = this.speed;
            this.sendControl({ type: 'speed', value: this.speed });
        });
    }

    initPipelineView() {
        // Initialize Konata pipeline view if the container exists
        const container = document.getElementById('pipelineViewContainer');
        console.log('CPUVisualization: Initializing pipeline view, container:', container);

        if (!container) {
            console.error('CPUVisualization: pipelineViewContainer not found');
            return;
        }

        if (typeof PipelineView === 'undefined') {
            console.error('CPUVisualization: PipelineView class not defined');
            container.innerHTML = '<div style="color: red; padding: 20px;">PipelineView not loaded</div>';
            return;
        }

        this.pipelineView = new PipelineView('pipelineViewContainer', {
            autoConnect: false  // We'll handle data through the main visualization
        });
        console.log('CPUVisualization: PipelineView initialized', this.pipelineView);
    }

    initTabs() {
        // Tab navigation
        const tabBtns = document.querySelectorAll('.tab-btn');
        tabBtns.forEach(btn => {
            btn.addEventListener('click', () => {
                const tabId = btn.dataset.tab;

                // Update active button
                tabBtns.forEach(b => b.classList.remove('active'));
                btn.classList.add('active');

                // Update active content
                document.querySelectorAll('.tab-content').forEach(content => {
                    content.classList.remove('active');
                });
                const tabContent = document.getElementById(`tab-${tabId}`);
                if (tabContent) {
                    tabContent.classList.add('active');
                }

                // Handle konata tab specifically
                if (tabId === 'konata') {
                    // Use requestAnimationFrame to ensure DOM has updated
                    requestAnimationFrame(() => {
                        // Initialize pipeline view if not already done
                        if (!this.pipelineView) {
                            console.log('CPUVisualization: Lazy initializing PipelineView');
                            this.initPipelineView();
                        }

                        // Another frame to ensure canvas is properly sized
                        requestAnimationFrame(() => {
                            if (this.pipelineView && this.pipelineView.renderer) {
                                console.log('CPUVisualization: Resizing renderer');
                                this.pipelineView.renderer.resize();
                                // Load data
                                this.loadKonataData();
                            }
                        });
                    });
                }
            });
        });
    }

    async loadInitialData() {
        try {
            const response = await fetch('/api/snapshots?limit=1000');
            const snapshots = await response.json();

            if (snapshots && snapshots.length > 0) {
                this.snapshots = snapshots;

                // Build IPC history from snapshots
                this.ipcHistory = snapshots.slice(-this.maxIpcHistory).map(s => ({
                    cycle: s.cycle,
                    ipc: s.metrics.ipc
                }));

                // Set current cycle to the latest
                this.currentCycle = snapshots[snapshots.length - 1].cycle;

                this.updateStatus(`Loaded ${snapshots.length} snapshots`, 'connected');
                this.render();
            }

            // Also load Konata data
            await this.loadKonataData();
        } catch (error) {
            console.error('Failed to load initial data:', error);
        }
    }

    async loadKonataData() {
        console.log('CPUVisualization: Loading Konata data, pipelineView:', this.pipelineView);
        if (!this.pipelineView) {
            console.warn('CPUVisualization: PipelineView not initialized');
            return;
        }

        try {
            const response = await fetch('/api/konata?limit=200');
            const snapshots = await response.json();

            console.log('CPUVisualization: Received Konata snapshots:', snapshots ? snapshots.length : 0);

            if (snapshots && snapshots.length > 0) {
                // Find the best snapshot: prefer ones with complete stage info (Issue, Execute/Memory)
                // Score each snapshot based on:
                // 1. Number of ops with Issue stage (Is)
                // 2. Number of ops with Execute or Memory stage (Ex/Me)
                const scoredSnapshots = snapshots.map(snap => {
                    if (!snap.ops || snap.ops.length === 0) return { snap, score: 0 };

                    let hasIssue = 0, hasExec = 0;
                    for (const op of snap.ops) {
                        const stages = op.lanes?.main?.stages || [];
                        const stageNames = stages.map(s => s.name);
                        if (stageNames.includes('Is')) hasIssue++;
                        if (stageNames.includes('Ex') || stageNames.includes('Me')) hasExec++;
                    }

                    // Score: prioritize ops with complete stages, then total ops
                    const score = hasExec * 1000 + hasIssue * 100 + snap.ops.length;
                    return { snap, score, hasIssue, hasExec, total: snap.ops.length };
                });

                // Sort by score descending and pick the best
                scoredSnapshots.sort((a, b) => b.score - a.score);
                const best = scoredSnapshots[0];

                console.log('CPUVisualization: Best snapshot (cycle', best.snap.cycle,
                    ') has', best.total, 'ops,', best.hasIssue, 'with Issue,', best.hasExec, 'with Exec/Mem');
                this.pipelineView.updateSnapshot(best.snap);
            }
        } catch (error) {
            console.error('CPUVisualization: Failed to load Konata data:', error);
        }
    }

    connect() {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/ws`;

        this.updateStatus('Connecting to server...');

        this.ws = new WebSocket(wsUrl);

        this.ws.onopen = () => {
            this.connected = true;
            this.updateStatus('Connected', 'connected');
        };

        this.ws.onclose = () => {
            this.connected = false;
            this.updateStatus('Disconnected - Reconnecting...', 'error');
            // Try to reconnect after 2 seconds
            setTimeout(() => this.connect(), 2000);
        };

        this.ws.onerror = (error) => {
            this.updateStatus('Connection error', 'error');
        };

        this.ws.onmessage = (event) => {
            try {
                const snapshot = JSON.parse(event.data);
                this.handleSnapshot(snapshot);
            } catch (e) {
                console.error('Failed to parse snapshot:', e);
            }
        };
    }

    updateStatus(message, type = '') {
        this.elements.connectionStatus.textContent = message;
        this.elements.connectionStatus.className = 'status-bar';
        if (type) {
            this.elements.connectionStatus.classList.add(type);
        }
    }

    sendControl(message) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(message));
        }
    }

    play() {
        this.isPlaying = true;
        this.sendControl({ type: 'play' });
    }

    pause() {
        this.isPlaying = false;
        this.sendControl({ type: 'pause' });
    }

    step() {
        this.isPlaying = false;
        this.sendControl({ type: 'step' });

        // Also advance locally
        if (this.snapshots.length > 0) {
            const maxCycle = Math.max(...this.snapshots.map(s => s.cycle));
            if (this.currentCycle < maxCycle) {
                this.currentCycle++;
                this.render();
            }
        }
    }

    reset() {
        this.isPlaying = false;
        this.currentCycle = 0;
        this.ipcHistory = [];
        this.sendControl({ type: 'reset' });
        this.render();

        // Reset pipeline view
        if (this.pipelineView) {
            this.pipelineView.clear();
        }
    }

    handleSnapshot(snapshot) {
        // Store snapshot
        this.snapshots.push(snapshot);
        if (this.snapshots.length > 10000) {
            this.snapshots.shift();
        }

        // Update IPC history
        this.ipcHistory.push({
            cycle: snapshot.cycle,
            ipc: snapshot.metrics.ipc
        });
        if (this.ipcHistory.length > this.maxIpcHistory) {
            this.ipcHistory.shift();
        }

        // Update current cycle if playing
        if (this.isPlaying) {
            this.currentCycle = snapshot.cycle;
        }

        // Render
        this.render();

        // Update Konata view periodically
        if (this.pipelineView && snapshot.cycle % 10 === 0) {
            this.fetchKonataSnapshot(snapshot.cycle);
        }
    }

    async fetchKonataSnapshot(cycle) {
        if (!this.pipelineView) return;

        try {
            const response = await fetch(`/api/konata/${cycle}`);
            const snapshot = await response.json();

            if (snapshot) {
                this.pipelineView.updateSnapshot(snapshot);
            }
        } catch (error) {
            // Silently ignore errors for Konata updates
        }
    }

    startAnimation() {
        const animate = (timestamp) => {
            if (this.isPlaying && this.snapshots.length > 0) {
                const elapsed = timestamp - this.lastUpdateTime;
                const interval = 1000 / this.speed;

                if (elapsed >= interval) {
                    // Advance to next snapshot if available
                    const maxCycle = Math.max(...this.snapshots.map(s => s.cycle));
                    if (this.currentCycle < maxCycle) {
                        this.currentCycle++;
                        this.render();
                    }
                    this.lastUpdateTime = timestamp;
                }
            }
            this.animationFrameId = requestAnimationFrame(animate);
        };
        this.animationFrameId = requestAnimationFrame(animate);
    }

    getCurrentSnapshot() {
        // Find snapshot for current cycle
        let snapshot = this.snapshots.find(s => s.cycle === this.currentCycle);
        if (!snapshot && this.snapshots.length > 0) {
            // Return the closest snapshot
            snapshot = this.snapshots.reduce((prev, curr) => {
                return Math.abs(curr.cycle - this.currentCycle) < Math.abs(prev.cycle - this.currentCycle) ? curr : prev;
            });
        }
        return snapshot;
    }

    render() {
        const snapshot = this.getCurrentSnapshot();
        if (!snapshot) {
            console.log('No snapshot to render');
            return;
        }

        this.renderMetrics(snapshot);
        this.renderPipeline(snapshot);
        this.renderInstructions(snapshot);
        this.renderDependencyGraph(snapshot);
        this.renderIpcChart();
        this.renderCacheStats(snapshot);
    }

    renderMetrics(snapshot) {
        this.elements.cycleValue.textContent = snapshot.cycle.toLocaleString();
        this.elements.committedValue.textContent = snapshot.committed_count.toLocaleString();
        this.elements.ipcValue.textContent = snapshot.metrics.ipc.toFixed(2);
        this.elements.l1HitValue.textContent = (snapshot.metrics.l1_hit_rate * 100).toFixed(1) + '%';
        this.elements.l2HitValue.textContent = (snapshot.metrics.l2_hit_rate * 100).toFixed(1) + '%';
        this.elements.windowValue.textContent = `${snapshot.pipeline.window_occupancy}/${snapshot.pipeline.window_capacity}`;
    }

    renderPipeline(snapshot) {
        this.elements.stageFetch.querySelector('.stage-count').textContent = snapshot.pipeline.fetch_count;
        this.elements.stageDispatch.querySelector('.stage-count').textContent = snapshot.pipeline.dispatch_count;
        this.elements.stageExecute.querySelector('.stage-count').textContent = snapshot.pipeline.execute_count;
        this.elements.stageMemory.querySelector('.stage-count').textContent = snapshot.pipeline.memory_count;
        this.elements.stageCommit.querySelector('.stage-count').textContent = snapshot.pipeline.commit_count;
    }

    renderInstructions(snapshot) {
        const tbody = this.elements.instructionBody;
        tbody.innerHTML = '';

        if (!snapshot.instructions || snapshot.instructions.length === 0) {
            const row = document.createElement('tr');
            row.innerHTML = '<td colspan="8" style="text-align: center; color: #666;">No instructions in window</td>';
            tbody.appendChild(row);
            return;
        }

        snapshot.instructions.forEach(instr => {
            const row = document.createElement('tr');

            // Format registers
            const srcRegs = instr.src_regs && instr.src_regs.length > 0
                ? instr.src_regs.map(r => `X${r}`).join(', ')
                : '-';
            const dstRegs = instr.dst_regs && instr.dst_regs.length > 0
                ? instr.dst_regs.map(r => `X${r}`).join(', ')
                : '-';

            // Format memory
            let mem = '-';
            if (instr.is_memory && instr.mem_addr !== null) {
                mem = `${instr.is_load ? 'R' : 'W'}: 0x${instr.mem_addr.toString(16)}`;
                if (instr.mem_size) {
                    mem += ` (${instr.mem_size}B)`;
                }
            }

            row.innerHTML = `
                <td>${instr.id}</td>
                <td>0x${instr.pc.toString(16).padStart(8, '0')}</td>
                <td>${instr.opcode}</td>
                <td><span class="status-badge ${instr.status}">${instr.status}</span></td>
                <td>${srcRegs}</td>
                <td>${dstRegs}</td>
                <td>${mem}</td>
                <td>${instr.pending_deps}</td>
            `;

            tbody.appendChild(row);
        });
    }

    renderDependencyGraph(snapshot) {
        const svg = this.elements.dependencyGraph;
        const width = svg.clientWidth || 600;
        const height = svg.clientHeight || 300;

        // Clear previous content
        svg.innerHTML = '';

        if (!snapshot.instructions || snapshot.instructions.length === 0) return;

        // Create a map of instruction IDs to their data
        const instrMap = new Map();
        snapshot.instructions.forEach((instr, i) => {
            instrMap.set(instr.id, { ...instr, index: i });
        });

        // Create nodes for instructions (limit to first 32 for performance)
        const maxNodes = 32;
        const nodes = snapshot.instructions.slice(0, maxNodes).map((instr, i) => ({
            id: instr.id,
            status: instr.status,
            opcode: instr.opcode,
            x: 50 + (i % 8) * (width - 100) / 7,
            y: 50 + Math.floor(i / 8) * 60
        }));

        // Create edges for dependencies
        const edges = (snapshot.dependencies || []).filter(dep =>
            instrMap.has(dep.from) && instrMap.has(dep.to)
        ).map(dep => {
            const fromNode = nodes.find(n => n.id === dep.from);
            const toNode = nodes.find(n => n.id === dep.to);
            return {
                source: fromNode,
                target: toNode,
                type: dep.dep_type
            };
        }).filter(e => e.source && e.target);

        // Draw edges
        edges.forEach(edge => {
            const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
            line.setAttribute('x1', edge.source.x);
            line.setAttribute('y1', edge.source.y);
            line.setAttribute('x2', edge.target.x);
            line.setAttribute('y2', edge.target.y);
            line.setAttribute('class', `link ${edge.type}`);
            svg.appendChild(line);
        });

        // Draw nodes
        nodes.forEach(node => {
            const g = document.createElementNS('http://www.w3.org/2000/svg', 'g');
            g.setAttribute('class', `node ${node.status}`);
            g.setAttribute('transform', `translate(${node.x}, ${node.y})`);

            const circle = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
            circle.setAttribute('r', '15');
            g.appendChild(circle);

            const text = document.createElementNS('http://www.w3.org/2000/svg', 'text');
            text.setAttribute('text-anchor', 'middle');
            text.setAttribute('dy', '4');
            text.textContent = node.id;
            g.appendChild(text);

            svg.appendChild(g);
        });
    }

    renderIpcChart() {
        const ctx = this.ipcCtx;
        const canvas = this.elements.ipcChart;
        const width = canvas.width;
        const height = canvas.height;

        // Clear canvas
        ctx.fillStyle = '#0f3460';
        ctx.fillRect(0, 0, width, height);

        if (this.ipcHistory.length < 2) {
            ctx.fillStyle = '#a0a0a0';
            ctx.font = '12px sans-serif';
            ctx.fillText('Waiting for data...', 10, height / 2);
            return;
        }

        // Find max IPC for scaling
        const maxIpc = Math.max(...this.ipcHistory.map(d => d.ipc), 1);

        // Draw grid
        ctx.strokeStyle = '#1a4a7a';
        ctx.lineWidth = 1;
        for (let i = 0; i <= 4; i++) {
            const y = height - (i / 4) * height;
            ctx.beginPath();
            ctx.moveTo(0, y);
            ctx.lineTo(width, y);
            ctx.stroke();
        }

        // Draw IPC line
        ctx.strokeStyle = '#e94560';
        ctx.lineWidth = 2;
        ctx.beginPath();

        this.ipcHistory.forEach((data, i) => {
            const x = (i / (this.ipcHistory.length - 1)) * width;
            const y = height - (data.ipc / maxIpc) * height;

            if (i === 0) {
                ctx.moveTo(x, y);
            } else {
                ctx.lineTo(x, y);
            }
        });

        ctx.stroke();

        // Draw labels
        ctx.fillStyle = '#a0a0a0';
        ctx.font = '10px sans-serif';
        ctx.fillText(`Max: ${maxIpc.toFixed(2)}`, 5, 15);
        ctx.fillText('0', 5, height - 5);
    }

    renderCacheStats(snapshot) {
        const metrics = snapshot.metrics;

        // Calculate percentages (simplified)
        const l1Hit = metrics.l1_hit_rate * 100;
        const l2Hit = metrics.l2_hit_rate * 100;
        const miss = 100 - l1Hit;

        this.elements.l1Bar.style.width = l1Hit + '%';
        this.elements.l2Bar.style.width = l2Hit + '%';
        this.elements.missBar.style.width = Math.max(0, miss) + '%';

        this.elements.l1Percent.textContent = l1Hit.toFixed(1) + '%';
        this.elements.l2Percent.textContent = l2Hit.toFixed(1) + '%';
        this.elements.missPercent.textContent = Math.max(0, miss).toFixed(1) + '%';
    }
}

// Initialize when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    window.visualization = new CPUVisualization();
});
