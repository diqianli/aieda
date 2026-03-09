//! SPEC17 Trace Validator for ARM CPU Emulator
//!
//! This tool validates the emulator by running SPEC CPU 2017 traces
//! and computing IPC, L1/L2 cache hit rates.
//!
//! Uses a proper out-of-order execution model with:
//! - Instruction window (ROB)
//! - Dependency tracking (register and memory)
//! - Memory-level parallelism (MLP)

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// ChampSim instruction format (64 bytes)
const INSTR_SIZE: usize = 64;

/// CPU Configuration
struct CPUConfig {
    // Out-of-order engine
    window_size: usize,        // ROB size
    issue_width: usize,        // Instructions per cycle
    commit_width: usize,       // Commit bandwidth

    // Latencies
    alu_latency: u64,          // ALU instruction latency
    l1_hit_latency: u64,       // L1 cache hit
    l2_hit_latency: u64,       // L2 cache hit
    l2_miss_latency: u64,      // Memory access

    // Memory subsystem
    l1_size: usize,
    l1_associativity: usize,
    l1_line_size: usize,
    l2_size: usize,
    l2_associativity: usize,
    l2_line_size: usize,

    // Load/Store Queue
    lsq_size: usize,
    mlp_limit: usize,          // Max outstanding memory requests
}

impl Default for CPUConfig {
    fn default() -> Self {
        Self {
            window_size: 256,      // 256 ROB
            issue_width: 8,        // 8-wide issue
            commit_width: 8,       // 8-wide commit
            alu_latency: 1,
            l1_hit_latency: 4,
            l2_hit_latency: 12,
            l2_miss_latency: 100,
            l1_size: 64 * 1024,
            l1_associativity: 4,
            l1_line_size: 64,
            l2_size: 512 * 1024,
            l2_associativity: 8,
            l2_line_size: 64,
            lsq_size: 64,
            mlp_limit: 32,         // 32 outstanding memory requests
        }
    }
}

/// Parsed ChampSim instruction
#[derive(Debug, Clone, Default)]
struct Instr {
    ip: u64,
    is_branch: bool,
    branch_taken: bool,
    destination_registers: [u8; 2],
    source_registers: [u8; 4],
    destination_memory: [u64; 2],
    source_memory: [u64; 4],
}

impl Instr {
    fn from_bytes(buf: &[u8; INSTR_SIZE]) -> Self {
        Self {
            ip: u64::from_le_bytes(buf[0..8].try_into().unwrap()),
            is_branch: buf[8] != 0,
            branch_taken: buf[9] != 0,
            destination_registers: [buf[10], buf[11]],
            source_registers: [buf[12], buf[13], buf[14], buf[15]],
            destination_memory: [
                u64::from_le_bytes(buf[16..24].try_into().unwrap()),
                u64::from_le_bytes(buf[24..32].try_into().unwrap()),
            ],
            source_memory: [
                u64::from_le_bytes(buf[32..40].try_into().unwrap()),
                u64::from_le_bytes(buf[40..48].try_into().unwrap()),
                u64::from_le_bytes(buf[48..56].try_into().unwrap()),
                u64::from_le_bytes(buf[56..64].try_into().unwrap()),
            ],
        }
    }

    fn is_load(&self) -> bool {
        self.source_memory.iter().any(|&x| x != 0)
    }

    fn is_store(&self) -> bool {
        self.destination_memory.iter().any(|&x| x != 0)
    }

    fn is_memory(&self) -> bool {
        self.is_load() || self.is_store()
    }

    fn load_addr(&self) -> Option<u64> {
        self.source_memory.iter().find(|&&x| x != 0).copied()
    }

    fn store_addr(&self) -> Option<u64> {
        self.destination_memory.iter().find(|&&x| x != 0).copied()
    }

    fn src_regs(&self) -> Vec<u8> {
        self.source_registers.iter().copied().filter(|&r| r != 0).collect()
    }

    fn dst_regs(&self) -> Vec<u8> {
        self.destination_registers.iter().copied().filter(|&r| r != 0).collect()
    }
}

/// Instruction in the ROB
#[derive(Debug, Clone)]
struct ROBEntry {
    instr_idx: usize,
    ready_cycle: u64,      // When all dependencies are ready
    issue_cycle: Option<u64>,
    complete_cycle: Option<u64>,
    latency: u64,
    is_memory: bool,
    mem_addr: Option<u64>,
    src_regs: Vec<u8>,
    dst_regs: Vec<u8>,
}

/// LRU cache simulator
struct Cache {
    name: String,
    associativity: usize,
    line_size: usize,
    sets: Vec<Vec<(u64, bool)>>,
    hits: u64,
    misses: u64,
}

impl Cache {
    fn new(name: &str, size: usize, associativity: usize, line_size: usize) -> Self {
        let num_sets = size / (associativity * line_size);
        let sets = vec![Vec::with_capacity(associativity); num_sets];

        Self {
            name: name.to_string(),
            associativity,
            line_size,
            sets,
            hits: 0,
            misses: 0,
        }
    }

    fn get_set_and_tag(&self, addr: u64) -> (usize, u64) {
        let line_bits = self.line_size.trailing_zeros() as u64;
        let set_mask = (self.sets.len() - 1) as u64;
        let set = ((addr >> line_bits) & set_mask) as usize;
        let tag = addr >> (line_bits + (self.sets.len().trailing_zeros() as u64));
        (set, tag)
    }

    fn access(&mut self, addr: u64) -> bool {
        let (set_idx, tag) = self.get_set_and_tag(addr);
        let set = &mut self.sets[set_idx];

        for i in 0..set.len() {
            if set[i].1 && set[i].0 == tag {
                let entry = set.remove(i);
                set.push(entry);
                self.hits += 1;
                return true;
            }
        }

        self.misses += 1;

        if set.len() >= self.associativity {
            set.remove(0);
        }
        set.push((tag, true));

        false
    }

    fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }

    fn mpki(&self, instructions: u64) -> f64 {
        if instructions == 0 { 0.0 } else { (self.misses as f64 / instructions as f64) * 1000.0 }
    }
}

/// Out-of-order CPU simulator with proper dependency tracking
struct OoOCPU {
    config: CPUConfig,
    l1: Cache,
    l2: Cache,

    // Register file: maps register -> cycle when ready
    reg_ready: HashMap<u8, u64>,

    // Memory dependency: maps address -> cycle when store completes
    mem_ready: HashMap<u64, u64>,

    // ROB
    rob: Vec<ROBEntry>,

    // Statistics
    total_instructions: u64,
    total_cycles: u64,
    memory_ops: u64,
    branch_ops: u64,
    branch_taken: u64,

    // Current cycle
    current_cycle: u64,

    // Outstanding memory requests
    outstanding_mem: usize,
}

impl OoOCPU {
    fn new(config: CPUConfig) -> Self {
        let l1 = Cache::new("L1", config.l1_size, config.l1_associativity, config.l1_line_size);
        let l2 = Cache::new("L2", config.l2_size, config.l2_associativity, config.l2_line_size);

        Self {
            config,
            l1,
            l2,
            reg_ready: HashMap::new(),
            mem_ready: HashMap::new(),
            rob: Vec::new(),
            total_instructions: 0,
            total_cycles: 0,
            memory_ops: 0,
            branch_ops: 0,
            branch_taken: 0,
            current_cycle: 0,
            outstanding_mem: 0,
        }
    }

    /// Calculate when instruction can execute based on dependencies
    fn calculate_ready_cycle(&self, instr: &Instr) -> u64 {
        let mut ready_cycle = self.current_cycle;

        // Check register dependencies
        for &reg in &instr.src_regs() {
            if let Some(&cycle) = self.reg_ready.get(&reg) {
                ready_cycle = ready_cycle.max(cycle);
            }
        }

        // Check memory dependencies (for loads, depend on previous stores to same address)
        if instr.is_load() {
            if let Some(addr) = instr.load_addr() {
                // Check exact address
                if let Some(&cycle) = self.mem_ready.get(&addr) {
                    ready_cycle = ready_cycle.max(cycle);
                }
                // Check same cache line (conservative)
                let line_mask = !(self.config.l1_line_size as u64 - 1);
                let line_addr = addr & line_mask;
                for (&store_addr, &cycle) in &self.mem_ready {
                    if (store_addr & line_mask) == line_addr && cycle > ready_cycle {
                        ready_cycle = cycle;
                    }
                }
            }
        }

        ready_cycle
    }

    /// Calculate memory access latency
    fn calculate_mem_latency(&mut self, addr: u64) -> u64 {
        let l1_hit = self.l1.access(addr);

        if l1_hit {
            self.config.l1_hit_latency
        } else {
            let l2_hit = self.l2.access(addr);
            if l2_hit {
                self.config.l2_hit_latency
            } else {
                self.config.l2_miss_latency
            }
        }
    }

    /// Dispatch instructions into the ROB
    fn dispatch(&mut self, instructions: &[Instr], start_idx: usize) -> usize {
        let mut dispatched = 0;

        while self.rob.len() < self.config.window_size && start_idx + dispatched < instructions.len() {
            let instr = &instructions[start_idx + dispatched];

            let ready_cycle = self.calculate_ready_cycle(instr);
            let is_memory = instr.is_memory();
            let mem_addr = if is_memory {
                instr.load_addr().or_else(|| instr.store_addr())
            } else {
                None
            };

            let latency = if is_memory {
                // Will be determined at issue time
                0
            } else {
                self.config.alu_latency
            };

            let entry = ROBEntry {
                instr_idx: start_idx + dispatched,
                ready_cycle,
                issue_cycle: None,
                complete_cycle: None,
                latency,
                is_memory,
                mem_addr,
                src_regs: instr.src_regs(),
                dst_regs: instr.dst_regs(),
            };

            self.rob.push(entry);
            dispatched += 1;
        }

        dispatched
    }

    /// Issue ready instructions (up to issue_width per cycle)
    fn issue(&mut self, _instructions: &[Instr]) {
        let mut issued = 0;
        let mlp_limit = self.config.mlp_limit;
        let alu_latency = self.config.alu_latency;
        let current_cycle = self.current_cycle;
        let outstanding_mem = self.outstanding_mem;

        // First pass: collect indices and data needed for issue
        let rob_len = self.rob.len();
        let mut to_issue: Vec<(usize, Option<u64>, bool)> = Vec::new(); // (index, mem_addr, is_memory)

        for idx in 0..rob_len {
            if issued >= self.config.issue_width {
                break;
            }

            let entry = &self.rob[idx];

            // Skip already issued
            if entry.issue_cycle.is_some() {
                continue;
            }

            // Check if ready
            if entry.ready_cycle > current_cycle {
                continue;
            }

            // For memory instructions, check MLP limit
            if entry.is_memory && outstanding_mem + to_issue.iter().filter(|(_, _, m)| *m).count() >= mlp_limit {
                continue;
            }

            to_issue.push((idx, entry.mem_addr, entry.is_memory));
            issued += 1;
        }

        // Second pass: calculate latencies (access caches)
        let mut latencies: Vec<(usize, u64, bool)> = Vec::new();
        for (idx, mem_addr, is_memory) in to_issue {
            let latency = if let Some(addr) = mem_addr {
                self.calculate_mem_latency(addr)
            } else {
                alu_latency
            };
            latencies.push((idx, latency, is_memory));
        }

        // Third pass: apply to ROB
        for (idx, latency, is_memory) in latencies {
            let entry = &mut self.rob[idx];
            entry.issue_cycle = Some(current_cycle);
            entry.latency = latency;
            entry.complete_cycle = Some(current_cycle + latency);

            if is_memory {
                self.outstanding_mem += 1;
            }
        }
    }

    /// Commit completed instructions (in-order)
    fn commit(&mut self, instructions: &[Instr]) -> bool {
        let mut committed = 0;
        let mut progress = false;

        while committed < self.config.commit_width && !self.rob.is_empty() {
            let entry = &self.rob[0];

            // Check if instruction is complete
            let complete_cycle = match entry.complete_cycle {
                Some(c) => c,
                None => break, // Not yet issued
            };

            if complete_cycle > self.current_cycle {
                break; // Not yet complete
            }

            // Commit this instruction
            let instr = &instructions[entry.instr_idx];

            // Update register ready times
            for &reg in &entry.dst_regs {
                self.reg_ready.insert(reg, complete_cycle);
            }

            // Update memory ready times for stores
            if instr.is_store() {
                if let Some(addr) = instr.store_addr() {
                    self.mem_ready.insert(addr, complete_cycle);
                }
            }

            // Track statistics
            self.total_instructions += 1;
            if entry.is_memory {
                self.memory_ops += 1;
                if self.outstanding_mem > 0 {
                    self.outstanding_mem -= 1;
                }
            }
            if instr.is_branch {
                self.branch_ops += 1;
                if instr.branch_taken {
                    self.branch_taken += 1;
                }
            }

            // Update total cycles
            self.total_cycles = self.total_cycles.max(complete_cycle);

            self.rob.remove(0);
            committed += 1;
            progress = true;
        }

        progress
    }

    /// Run simulation on a batch of instructions
    fn run(&mut self, instructions: &[Instr]) {
        let mut dispatch_idx = 0;

        // Initial dispatch
        dispatch_idx += self.dispatch(instructions, dispatch_idx);

        // Main simulation loop
        while !self.rob.is_empty() || dispatch_idx < instructions.len() {
            // Issue ready instructions
            self.issue(instructions);

            // Try to commit
            let progress = self.commit(instructions);

            // Dispatch more if there's room
            if self.rob.len() < self.config.window_size {
                dispatch_idx += self.dispatch(instructions, dispatch_idx);
            }

            // Advance cycle
            self.current_cycle += 1;

            // Safety check to avoid infinite loops
            if !progress && self.rob.iter().all(|e| e.issue_cycle.is_none()) {
                // All waiting - something is wrong
                break;
            }
        }

        // Make sure we account for the final cycle
        if self.total_cycles == 0 {
            self.total_cycles = self.current_cycle;
        }
    }

    fn ipc(&self) -> f64 {
        if self.total_cycles == 0 { 0.0 } else { self.total_instructions as f64 / self.total_cycles as f64 }
    }

    fn cpi(&self) -> f64 {
        if self.total_instructions == 0 { 0.0 } else { self.total_cycles as f64 / self.total_instructions as f64 }
    }
}

/// Read compressed trace
fn read_xz_trace<P: AsRef<Path>>(path: P, max_instructions: u64) -> std::io::Result<Vec<Instr>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut decoder = xz2::read::XzDecoder::new(reader);

    let mut buffer = [0u8; INSTR_SIZE];
    let mut instructions = Vec::with_capacity(max_instructions as usize);

    while instructions.len() < max_instructions as usize {
        let mut total_read = 0;
        while total_read < INSTR_SIZE {
            match decoder.read(&mut buffer[total_read..]) {
                Ok(0) => {
                    if total_read == 0 {
                        return Ok(instructions);
                    } else {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "Unexpected end of file",
                        ));
                    }
                }
                Ok(n) => total_read += n,
                Err(e) => return Err(e),
            }
        }

        instructions.push(Instr::from_bytes(&buffer));
    }

    Ok(instructions)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("SPEC17 Trace Validator for ARM CPU Emulator (Out-of-Order Model)");
        println!("================================================================");
        println!();
        println!("Usage: {} <trace_file.xz> [max_instructions]", args[0]);
        println!();
        println!("Example:");
        println!("  {} 600.perlbench_s-210B.champsimtrace.xz 1000000", args[0]);
        println!();
        println!("Available traces in /Users/mac/storage/Champsim/traces/:");
        println!("  - 600.perlbench_s-210B.champsimtrace.xz");
        println!("  - 605.mcf_s-472B.champsimtrace.xz");
        println!("  - 619.lbm_s-2676B.champsimtrace.xz");
        std::process::exit(1);
    }

    let trace_path = &args[1];
    let max_instructions: u64 = if args.len() > 2 {
        args[2].parse().unwrap_or(1_000_000)
    } else {
        1_000_000
    };

    let config = CPUConfig::default();

    println!("SPEC17 Trace Validator (Out-of-Order Model)");
    println!("============================================");
    println!();
    println!("CPU Configuration:");
    println!("  ROB Size: {}", config.window_size);
    println!("  Issue Width: {}", config.issue_width);
    println!("  Commit Width: {}", config.commit_width);
    println!("  L1 Latency: {} cycles", config.l1_hit_latency);
    println!("  L2 Latency: {} cycles", config.l2_hit_latency);
    println!("  Memory Latency: {} cycles", config.l2_miss_latency);
    println!("  MLP Limit: {} outstanding", config.mlp_limit);
    println!();
    println!("Trace file: {}", trace_path);
    println!("Max instructions: {}", max_instructions);
    println!();

    // Read trace
    println!("Loading trace (this may take a while)...");
    let instructions = match read_xz_trace(trace_path, max_instructions) {
        Ok(instrs) => instrs,
        Err(e) => {
            eprintln!("Error reading trace: {}", e);
            std::process::exit(1);
        }
    };

    println!("Loaded {} instructions", instructions.len());
    println!();

    // Run simulation
    println!("Running out-of-order simulation...");
    let mut cpu = OoOCPU::new(config);
    cpu.run(&instructions);

    // Print results
    println!();
    println!("Simulation Results");
    println!("==================");
    println!();
    println!("Execution Statistics:");
    println!("  Total Instructions: {}", cpu.total_instructions);
    println!("  Total Cycles: {}", cpu.total_cycles);
    println!("  IPC: {:.4}", cpu.ipc());
    println!("  CPI: {:.4}", cpu.cpi());
    println!();
    println!("Instruction Mix:");
    println!("  Memory Operations: {} ({:.2}%)",
        cpu.memory_ops,
        cpu.memory_ops as f64 / cpu.total_instructions as f64 * 100.0
    );
    println!("  Branch Operations: {} ({:.2}%)",
        cpu.branch_ops,
        cpu.branch_ops as f64 / cpu.total_instructions as f64 * 100.0
    );
    println!("  Branches Taken: {} ({:.2}% of branches)",
        cpu.branch_taken,
        if cpu.branch_ops > 0 {
            cpu.branch_taken as f64 / cpu.branch_ops as f64 * 100.0
        } else {
            0.0
        }
    );
    println!();
    println!("L1 Cache (64KB, 4-way):");
    println!("  Accesses: {}", cpu.l1.hits + cpu.l1.misses);
    println!("  Hits: {}", cpu.l1.hits);
    println!("  Misses: {}", cpu.l1.misses);
    println!("  Hit Rate: {:.2}%", cpu.l1.hit_rate() * 100.0);
    println!("  MPKI: {:.2}", cpu.l1.mpki(cpu.total_instructions));
    println!();
    println!("L2 Cache (512KB, 8-way):");
    println!("  Accesses: {}", cpu.l2.hits + cpu.l2.misses);
    println!("  Hits: {}", cpu.l2.hits);
    println!("  Misses: {}", cpu.l2.misses);
    println!("  Hit Rate: {:.2}%", cpu.l2.hit_rate() * 100.0);
    println!("  MPKI: {:.2}", cpu.l2.mpki(cpu.total_instructions));
    println!();

    // Summary JSON
    let results = serde_json::json!({
        "trace_file": trace_path,
        "model": "out_of_order",
        "config": {
            "rob_size": cpu.config.window_size,
            "issue_width": cpu.config.issue_width,
            "commit_width": cpu.config.commit_width,
            "l1_latency": cpu.config.l1_hit_latency,
            "l2_latency": cpu.config.l2_hit_latency,
            "mem_latency": cpu.config.l2_miss_latency,
            "mlp_limit": cpu.config.mlp_limit,
        },
        "total_instructions": cpu.total_instructions,
        "total_cycles": cpu.total_cycles,
        "ipc": cpu.ipc(),
        "cpi": cpu.cpi(),
        "memory_ops": cpu.memory_ops,
        "memory_ops_pct": cpu.memory_ops as f64 / cpu.total_instructions as f64 * 100.0,
        "branch_ops": cpu.branch_ops,
        "branch_ops_pct": cpu.branch_ops as f64 / cpu.total_instructions as f64 * 100.0,
        "l1": {
            "hits": cpu.l1.hits,
            "misses": cpu.l1.misses,
            "hit_rate": cpu.l1.hit_rate(),
            "mpki": cpu.l1.mpki(cpu.total_instructions)
        },
        "l2": {
            "hits": cpu.l2.hits,
            "misses": cpu.l2.misses,
            "hit_rate": cpu.l2.hit_rate(),
            "mpki": cpu.l2.mpki(cpu.total_instructions)
        }
    });

    println!("JSON Results:");
    println!("{}", serde_json::to_string_pretty(&results).unwrap());
}
