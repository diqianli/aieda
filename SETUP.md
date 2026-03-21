# ARM CPU Emulator - Setup and Usage Guide

Complete guide for setting up and reproducing the ARM CPU emulator with Konata visualization and TopDown analysis.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Quick Start](#quick-start)
3. [Project Structure](#project-structure)
4. [Building](#building)
5. [Running Simulations](#running-simulations)
6. [Visualization](#visualization)
7. [TopDown Analysis](#topdown-analysis)
8. [Configuration Options](#configuration-options)
9. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### System Requirements

- **OS**: macOS 10.15+, Ubuntu 20.04+, or Windows 10+ with WSL2
- **RAM**: 4GB minimum (8GB recommended for large simulations)
- **Disk**: 2GB free space
- **Rust**: 1.70.0 or later

### Required Tools

```bash
# Check Rust version
rustc --version

# Install Rust if needed (recommended via rustup)
curl --proto '=https' --tlsv1.2' -sSf https://sh.rustup.rs/rustup-init.sh | sh
source ~/.cargo/env
rustup install stable
```

### Dependencies

The project uses the following major dependencies (managed by Cargo):

- `serde` - JSON serialization
- `serde_json` - JSON handling
- `ahash` - Fast hash maps
- `tracing` - Logging framework
- `schemars` - JSON schema generation

---

## Quick Start

### 1. Clone the Repository

```bash
# Using HTTPS
git clone https://github.com/diqianli/aieda.git
cd aieda

# Or using SSH
git clone git@github.com:diqianli/aieda.git
cd aieda
```

### 2. Build the Project

```bash
# Debug build
cargo build

# Release build (recommended for simulations)
cargo build --release
```

### 3. Run a Test Simulation

```bash
# Generate 1000 instructions with visualization
cargo run --features visualization --example generate_konata 1000

# Output files will be in visualization/static/
```

### 4. View the Visualization

```bash
# Start HTTP server
cd visualization/static && python3 -m http.server 8080

# Open browser to:
# - http://localhost:8080/index_static.html (Pipeline visualization)
# - http://localhost:8080/konata_data_report.html (TopDown analysis)
```

---

## Project Structure

```
arm_cpu_emulator/
├── Cargo.toml              # Project dependencies and features
├── Cargo.lock              # Locked dependency versions
├── .gitignore             # Git ignore rules
├── SETUP.md               # This file
│
├── src/
│   ├── lib.rs             # Library entry point
│   ├── cpu.rs             # CPU emulator main module
│   ├── types.rs           # Core type definitions
│   ├── config.rs          # Configuration structures
│   │
│   ├── analysis/          # Performance analysis modules
│   │   ├── mod.rs
│   │   ├── topdown.rs     # TopDown analysis (Intel methodology)
│   │   ├── aggregator.rs  # Statistics aggregation
│   │   └── function_profiler.rs
│   │
│   ├── decoder/           # Instruction decoders
│   │   ├── mod.rs
│   │   └── aarch64/       # ARM64 instruction decoding
│   │
│   ├── elf/               # ELF file handling
│   │   ├── loader.rs
│   │   └── decoder.rs
│   │
│   ├── memory/            # Memory subsystem
│   │   ├── mod.rs
│   │   ├── cache.rs
│   │   └── enhanced.rs    # Enhanced cache with prefetcher
│   │
│   ├── ooo/               # Out-of-order execution
│   │   ├── mod.rs
│   │   ├── window.rs      # Instruction window
│   │   └── scheduler.rs   # Issue scheduler
│   │
│   ├── simulation/        # Simulation engine
│   │   ├── engine.rs
│   │   ├── event.rs
│   │   └── tracker.rs
│   │
│   ├── output/            # Output formats
│   │   ├── konata.rs      # Konata format generator
│   │   └── sink.rs        # Output sink trait
│   │
│   ├── trace/             # Trace file handling
│   │   ├── binary_format.rs
│   │   ├── binary_reader.rs
│   │   └── binary_writer.rs
│   │
│   └── visualization/     # Visualization support
│       ├── mod.rs
│       ├── pipeline_tracker.rs
│       └── konata_format.rs
│
├── examples/               # Example programs
│   ├── generate_konata.rs # Generate Konata JSON + TopDown report
│   ├── elf_to_konata.rs   # Convert ELF to Konata
│   └── test_*.rs          # Test examples
│
├── visualization/
│   └── static/             # Web visualization files
│       ├── index_static.html
│       ├── app_static.js
│       ├── pipeline_view.js
│       ├── style.css
│       └── konata/         # Konata renderer
│           ├── op.js
│           ├── stage.js
│           └── konata_renderer.js
│
└── test_programs/          # Test ELF binaries
```

---

## Building

### Debug Build

```bash
cargo build
```

### Release Build (Recommended)

```bash
cargo build --release
```

### With Visualization Feature

```bash
# Required for Konata visualization output
cargo build --features visualization
```

### All Features

```bash
cargo build --all-features
```

---

## Running Simulations

### Basic Simulation

```bash
# Run with default settings (500 instructions)
cargo run --features visualization --example generate_konata

# Run with custom instruction count
cargo run --features visualization --example generate_konata 10000

# Run with custom output path
cargo run --features visualization --example generate_konata 100000 /path/to/output.json
```

### Simulation with ELF File

```bash
# Convert ELF to Konata visualization
cargo run --features visualization --example elf_to_konata -- program.elf output.json
```

### Expected Output

After running `generate_konata`, the following files are generated:

```
visualization/static/
├── konata_data.json          # Konata pipeline data (large file)
├── konata_data_topdown.json  # TopDown analysis JSON
└── konata_data_report.html   # HTML visualization report
```

### Simulation Metrics

The simulation outputs key metrics:

```
Total cycles: 100001
Instructions committed: 100000
IPC: 1.00
```

---

## Visualization

### Starting the Visualization Server

```bash
# From the project root
cd visualization/static && python3 -m http.server 8080

# Or using Node.js
npx http-server visualization/static -p 8080
```

### Accessing the Visualization

Open your browser to:

| Page | URL | Description |
|------|-----|-------------|
| Pipeline View | http://localhost:8080/index_static.html | Konata pipeline visualization |
| TopDown Report | http://localhost:8080/konata_data_report.html | TopDown analysis with charts |

### Pipeline Visualization Features

- **Pan**: Click and drag to scroll horizontally
- **Zoom**: Use mouse wheel or pinch gesture
- **Click instruction**: Shows details in tooltip
- **Double-click**: Aligns view to instruction

### TopDown Report Features

- **Summary Metrics**: IPC, total cycles, instructions
- **TopDown Level 1**: Retiring, Frontend Bound, Backend Bound, Bad Speculation
- **Instruction Mix**: ALU, Memory, Branch, SIMD breakdown
- **Stage Utilization**: Pipeline stage usage chart
- **Cycle Distribution**: Full issue, partial issue, stall cycles
- **Hotspots**: Top 20 PC ranges by cycle count

---

## TopDown Analysis

### Understanding TopDown Metrics

The TopDown methodology (Intel) identifies performance bottlenecks:

| Metric | Description | Target |
|--------|-------------|--------|
| **Retiring** | Useful work completing successfully | High (>50%) |
| **Frontend Bound** | Fetch/decode bottlenecks | Low (<10%) |
| **Backend Bound** | Execution/memory bottlenecks | Low (<20%) |
| **Bad Speculation** | Branch misprediction waste | Low (<10%) |

### Generating TopDown Report

The TopDown report is automatically generated when running `generate_konata`:

```bash
cargo run --features visualization --example generate_konata 100000
```

### TopDown JSON Structure

```json
{
  "version": "1.0",
  "summary": {
    "total_cycles": 100001,
    "total_instructions": 100000,
    "ipc": 1.0,
    "issue_width": 6,
    "window_size": 256
  },
  "topdown": {
    "retiring_pct": 16.7,
    "bad_speculation_pct": 56.7,
    "frontend_bound_pct": 10.7,
    "backend_bound_pct": 16.0
  },
  "hotspots": [...],
  "cycle_distribution": {...}
}
```

---

## Configuration Options

### CPU Configuration

```rust
use arm_cpu_emulator::CPUConfig;

let config = CPUConfig {
    // Instruction window size
    window_size: 256,

    // Issue width (instructions per cycle)
    issue_width: 6,

    // Commit width (retire per cycle)
    commit_width: 6,

    // Fetch width (fetch per cycle)
    fetch_width: 8,

    // Cache sizes
    l1_size: 64 * 1024,      // 64KB L1
    l2_size: 512 * 1024,     // 512KB L2

    // Enable trace output
    enable_trace_output: false,

    ..Default::default()
};
```

### Visualization Configuration

```rust
use arm_cpu_emulator::VisualizationConfig;

let viz_config = VisualizationConfig {
    enabled: true,
    port: 3000,
    max_snapshots: 10,
    animation_speed: 10,
};
```

---

## Troubleshooting

### Common Issues

#### 1. "Visualization feature not enabled"

```bash
# Add --features visualization flag
cargo run --features visualization --example generate_konata
```

#### 2. "konata_data.json too large for GitHub"

The large JSON file is excluded from git via `.gitignore`. Generate it locally:

```bash
cargo run --features visualization --example generate_konata 100000
```

#### 3. "Port 8080 already in use"

```bash
# Find and kill existing process
lsof -i :8080
kill -9 <PID>

# Or use different port
python3 -m http.server 8081
```

#### 4. "Blank visualization page"

- Check browser console for JavaScript errors (F12 → Console)
- Ensure `konata_data.json` exists in `visualization/static/`
- Try hard refresh (Cmd+Shift+R on Mac)

#### 5. "Charts not loading in TopDown report"

The report requires internet access to load Chart.js from CDN.
If offline, download Chart.js locally and modify the HTML.

### Debug Logging

Enable trace logging:

```bash
RUST_LOG=debug cargo run --features visualization --example generate_konata
```

### Memory Issues

For large simulations, increase system memory or reduce instruction count:

```bash
# Instead of 1M instructions
cargo run --features visualization --example generate_konata 10000
```

---

## GitHub SSH Setup

### Generate SSH Key (if not exists)

```bash
ssh-keygen -t ed25519 -C "your_email@example.com"
```

### Add Key to SSH Agent

```bash
eval "$(ssh-agent -s)"
ssh-add ~/.ssh/id_ed25519
```

### Add Public Key to GitHub

1. Copy public key:
   ```bash
   cat ~/.ssh/id_ed25519.pub
   ```

2. Go to GitHub → Settings → SSH and GPG keys → New SSH key

3. Paste the public key and save

### Test SSH Connection

```bash
ssh -T git@github.com
# Should show: Hi username! You've successfully authenticated...
```

### Use SSH URL for Git Remote

```bash
git remote set-url origin git@github.com:username/repo.git
```

---

## License

MIT License - See LICENSE file for details.

---

## Support

For issues or questions:
- Open an issue on GitHub: https://github.com/diqianli/aieda/issues
- Check the project documentation in `docs/` directory
