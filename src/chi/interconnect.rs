//! CHI Interconnect - connects RN-F, HN-F, and SN-F nodes.

use std::collections::VecDeque;

use super::node::{
    ChiRequest, ChiResponse, ChiData, ChiSnoop, ChiSnoopResp,
    NodeId,
};
use super::rn_f::RnFNode;
use super::hn_f::HnFNode;
use super::sn_f::SnFNode;

/// CHI Interconnect configuration
#[derive(Debug, Clone)]
pub struct ChiInterconnectConfig {
    /// Request channel latency
    pub req_latency: u64,
    /// Response channel latency
    pub rsp_latency: u64,
    /// Data channel latency
    pub dat_latency: u64,
    /// Snoop channel latency
    pub snp_latency: u64,
}

impl Default for ChiInterconnectConfig {
    fn default() -> Self {
        Self {
            req_latency: 2,
            rsp_latency: 2,
            dat_latency: 4,
            snp_latency: 2,
        }
    }
}

/// Message in flight through the interconnect
#[derive(Debug, Clone)]
pub struct InFlightMessage {
    /// Destination node ID
    pub dest: NodeId,
    /// Cycle when message arrives
    pub arrival_cycle: u64,
    /// Message type
    pub msg_type: InFlightMessageType,
}

/// Types of messages that can be in flight
#[derive(Debug, Clone)]
pub enum InFlightMessageType {
    Request(ChiRequest),
    Response(ChiResponse),
    Data(ChiData),
    Snoop(ChiSnoop),
    SnoopResp(ChiSnoopResp),
}

/// CHI Interconnect
pub struct ChiInterconnect {
    /// Configuration
    config: ChiInterconnectConfig,
    /// Messages in flight
    in_flight: VecDeque<InFlightMessage>,
    /// Current cycle
    current_cycle: u64,
    /// Statistics
    stats: InterconnectStats,
}

/// Interconnect statistics
#[derive(Debug, Clone, Default)]
pub struct InterconnectStats {
    /// Requests transmitted
    pub requests: u64,
    /// Responses transmitted
    pub responses: u64,
    /// Data messages transmitted
    pub data_msgs: u64,
    /// Snoop messages transmitted
    pub snoops: u64,
    /// Snoop responses transmitted
    pub snoop_resps: u64,
    /// Total messages
    pub total_messages: u64,
    /// Peak in-flight messages
    pub peak_in_flight: usize,
}

impl ChiInterconnect {
    /// Create a new interconnect
    pub fn new(config: ChiInterconnectConfig) -> Self {
        Self {
            config,
            in_flight: VecDeque::new(),
            current_cycle: 0,
            stats: InterconnectStats::default(),
        }
    }

    /// Send a request
    pub fn send_request(&mut self, req: ChiRequest, dest: NodeId) {
        let arrival = self.current_cycle + self.config.req_latency;
        self.in_flight.push_back(InFlightMessage {
            dest,
            arrival_cycle: arrival,
            msg_type: InFlightMessageType::Request(req),
        });
        self.stats.requests += 1;
        self.stats.total_messages += 1;
        self.update_peak();
    }

    /// Send a response
    pub fn send_response(&mut self, resp: ChiResponse, dest: NodeId) {
        let arrival = self.current_cycle + self.config.rsp_latency;
        self.in_flight.push_back(InFlightMessage {
            dest,
            arrival_cycle: arrival,
            msg_type: InFlightMessageType::Response(resp),
        });
        self.stats.responses += 1;
        self.stats.total_messages += 1;
        self.update_peak();
    }

    /// Send a data message
    pub fn send_data(&mut self, data: ChiData, dest: NodeId) {
        let arrival = self.current_cycle + self.config.dat_latency;
        self.in_flight.push_back(InFlightMessage {
            dest,
            arrival_cycle: arrival,
            msg_type: InFlightMessageType::Data(data),
        });
        self.stats.data_msgs += 1;
        self.stats.total_messages += 1;
        self.update_peak();
    }

    /// Send a snoop
    pub fn send_snoop(&mut self, snoop: ChiSnoop, dest: NodeId) {
        let arrival = self.current_cycle + self.config.snp_latency;
        self.in_flight.push_back(InFlightMessage {
            dest,
            arrival_cycle: arrival,
            msg_type: InFlightMessageType::Snoop(snoop),
        });
        self.stats.snoops += 1;
        self.stats.total_messages += 1;
        self.update_peak();
    }

    /// Send a snoop response
    pub fn send_snoop_resp(&mut self, resp: ChiSnoopResp, dest: NodeId) {
        let arrival = self.current_cycle + self.config.snp_latency;
        self.in_flight.push_back(InFlightMessage {
            dest,
            arrival_cycle: arrival,
            msg_type: InFlightMessageType::SnoopResp(resp),
        });
        self.stats.snoop_resps += 1;
        self.stats.total_messages += 1;
        self.update_peak();
    }

    /// Update peak in-flight tracking
    fn update_peak(&mut self) {
        if self.in_flight.len() > self.stats.peak_in_flight {
            self.stats.peak_in_flight = self.in_flight.len();
        }
    }

    /// Get messages arriving this cycle
    pub fn get_arriving_messages(&mut self) -> Vec<(NodeId, InFlightMessageType)> {
        let mut arriving = Vec::new();

        while let Some(msg) = self.in_flight.front() {
            if msg.arrival_cycle <= self.current_cycle {
                let msg = self.in_flight.pop_front().unwrap();
                arriving.push((msg.dest, msg.msg_type));
            } else {
                break;
            }
        }

        arriving
    }

    /// Advance simulation by one cycle
    pub fn advance_cycle(&mut self) {
        self.current_cycle += 1;
    }

    /// Get current cycle
    pub fn current_cycle(&self) -> u64 {
        self.current_cycle
    }

    /// Get statistics
    pub fn stats(&self) -> &InterconnectStats {
        &self.stats
    }

    /// Get number of in-flight messages
    pub fn in_flight_count(&self) -> usize {
        self.in_flight.len()
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.in_flight.clear();
    }
}

/// CHI System - complete CHI system with all nodes
pub struct ChiSystem {
    /// RN-F nodes (requesters)
    pub rn_f_nodes: Vec<RnFNode>,
    /// HN-F node (home)
    pub hn_f: HnFNode,
    /// SN-F node (memory)
    pub sn_f: SnFNode,
    /// Interconnect
    pub interconnect: ChiInterconnect,
    /// Current cycle
    current_cycle: u64,
}

impl ChiSystem {
    /// Create a new CHI system with a single RN-F
    pub fn new_single_core(
        rn_f: RnFNode,
        hn_f: HnFNode,
        sn_f: SnFNode,
        interconnect_config: ChiInterconnectConfig,
    ) -> Self {
        Self {
            rn_f_nodes: vec![rn_f],
            hn_f,
            sn_f,
            interconnect: ChiInterconnect::new(interconnect_config),
            current_cycle: 0,
        }
    }

    /// Create a new CHI system with multiple RN-F nodes
    pub fn new_multi_core(
        rn_f_nodes: Vec<RnFNode>,
        hn_f: HnFNode,
        sn_f: SnFNode,
        interconnect_config: ChiInterconnectConfig,
    ) -> Self {
        Self {
            rn_f_nodes,
            hn_f,
            sn_f,
            interconnect: ChiInterconnect::new(interconnect_config),
            current_cycle: 0,
        }
    }

    /// Process all interconnect traffic
    pub fn process_interconnect(&mut self) {
        // Collect outgoing messages from all nodes

        // From RN-F nodes to HN-F
        for rn_f in &mut self.rn_f_nodes {
            // Requests
            while let Some(req) = rn_f.req_channel_mut().recv() {
                self.interconnect.send_request(req, self.hn_f.node_id);
            }

            // Snoop responses
            while let Some(resp) = rn_f.snp_resp_channel_mut().recv() {
                self.interconnect.send_snoop_resp(resp, self.hn_f.node_id);
            }
        }

        // From HN-F to RN-F and SN-F
        // Responses
        while let Some(resp) = self.hn_f.rsp_channel_mut().recv() {
            let dest = NodeId(resp.dest_id.0);
            self.interconnect.send_response(resp, dest);
        }

        // Data
        while let Some(data) = self.hn_f.dat_channel_mut().recv() {
            let dest = data.dest_id;
            self.interconnect.send_data(data, dest);
        }

        // Snoops
        while let Some(snoop) = self.hn_f.snp_channel_mut().recv() {
            let dest = snoop.dest_id;
            self.interconnect.send_snoop(snoop, dest);
        }

        // Requests to SN-F
        while let Some(req) = self.hn_f.sn_req_channel_mut().recv() {
            self.interconnect.send_request(req, self.sn_f.node_id);
        }

        // From SN-F to HN-F
        // Responses
        while let Some(resp) = self.sn_f.rsp_channel_mut().recv() {
            let dest = NodeId(resp.dest_id.0);
            self.interconnect.send_response(resp, dest);
        }

        // Data
        while let Some(data) = self.sn_f.dat_channel_mut().recv() {
            let dest = data.dest_id;
            self.interconnect.send_data(data, dest);
        }

        // Deliver arriving messages
        let arriving = self.interconnect.get_arriving_messages();
        for (dest, msg_type) in arriving {
            self.deliver_message(dest, msg_type);
        }
    }

    /// Deliver a message to a node
    fn deliver_message(&mut self, dest: NodeId, msg_type: InFlightMessageType) {
        match msg_type {
            InFlightMessageType::Request(req) => {
                if dest == self.hn_f.node_id {
                    self.hn_f.req_channel_mut().send(req);
                } else if dest == self.sn_f.node_id {
                    self.sn_f.req_channel_mut().send(req);
                }
            }
            InFlightMessageType::Response(resp) => {
                if let Some(rn_f) = self.find_rn_f(dest) {
                    rn_f.rsp_channel_mut().send(resp);
                } else if dest == self.hn_f.node_id {
                    self.hn_f.rsp_channel_mut().send(resp);
                }
            }
            InFlightMessageType::Data(data) => {
                let data_dest = data.dest_id;
                if let Some(rn_f) = self.find_rn_f(data_dest) {
                    rn_f.dat_channel_mut().send(data);
                } else if data_dest == self.hn_f.node_id {
                    self.hn_f.sn_dat_channel_mut().send(data);
                }
            }
            InFlightMessageType::Snoop(snoop) => {
                let snoop_dest = snoop.dest_id;
                if let Some(rn_f) = self.find_rn_f(snoop_dest) {
                    rn_f.snp_channel_mut().send(snoop);
                }
            }
            InFlightMessageType::SnoopResp(resp) => {
                if dest == self.hn_f.node_id {
                    self.hn_f.snp_resp_channel_mut().send(resp);
                }
            }
        }
    }

    /// Find an RN-F node by ID
    fn find_rn_f(&mut self, node_id: NodeId) -> Option<&mut RnFNode> {
        self.rn_f_nodes.iter_mut().find(|n| n.node_id == node_id)
    }

    /// Process all nodes
    pub fn process_nodes(&mut self) {
        // Process RN-F nodes
        for rn_f in &mut self.rn_f_nodes {
            // Handle incoming snoops
            while let Some(snoop) = rn_f.snp_channel_mut().recv() {
                rn_f.handle_snoop(snoop);
            }

            // Handle incoming responses
            while let Some(resp) = rn_f.rsp_channel_mut().recv() {
                rn_f.handle_response(resp);
            }

            // Handle incoming data
            while let Some(data) = rn_f.dat_channel_mut().recv() {
                rn_f.handle_data(data);
            }

            // Process retries
            rn_f.process_retries();
        }

        // Process HN-F
        self.hn_f.process_requests();
        self.hn_f.process_snoop_responses();
        self.hn_f.process_memory_responses();

        // Process SN-F
        self.sn_f.process_requests();
        self.sn_f.process_completions();
    }

    /// Advance simulation by one cycle
    pub fn advance_cycle(&mut self) {
        self.current_cycle += 1;

        for rn_f in &mut self.rn_f_nodes {
            rn_f.advance_cycle();
        }
        self.hn_f.advance_cycle();
        self.sn_f.advance_cycle();
        self.interconnect.advance_cycle();
    }

    /// Run one complete simulation step
    pub fn step(&mut self) {
        self.process_nodes();
        self.process_interconnect();
        self.advance_cycle();
    }

    /// Get current cycle
    pub fn current_cycle(&self) -> u64 {
        self.current_cycle
    }

    /// Get the primary RN-F node (for single-core systems)
    pub fn primary_rn_f(&self) -> Option<&RnFNode> {
        self.rn_f_nodes.first()
    }

    /// Get mutable access to primary RN-F node
    pub fn primary_rn_f_mut(&mut self) -> Option<&mut RnFNode> {
        self.rn_f_nodes.first_mut()
    }

    /// Check if there are any pending transactions
    pub fn has_pending_transactions(&self) -> bool {
        self.rn_f_nodes.iter().any(|n| n.has_pending_transactions())
            || self.hn_f.has_pending_transactions()
            || self.sn_f.has_pending_requests()
            || self.interconnect.in_flight_count() > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::protocol::{ChiTxnId, ChiRequestType};
    use crate::types::InstructionId;

    #[test]
    fn test_interconnect_basic() {
        let mut ic = ChiInterconnect::new(ChiInterconnectConfig::default());

        let req = ChiRequest::new(
            ChiTxnId::new(1),
            NodeId(0),
            NodeId(1),
            ChiRequestType::ReadShared,
            0x1000,
            64,
            InstructionId(0),
        );

        ic.send_request(req, NodeId(1));
        assert_eq!(ic.in_flight_count(), 1);

        // Message should not arrive immediately
        let arriving = ic.get_arriving_messages();
        assert!(arriving.is_empty());

        // Advance time
        ic.advance_cycle();
        ic.advance_cycle();

        // Now message should arrive
        let arriving = ic.get_arriving_messages();
        assert_eq!(arriving.len(), 1);
    }
}
