# ARM CPU Emulator

ARMv8-A (AArch64) CPU emulator with out-of-order execution simulation and Konata-compatible pipeline visualization.

## Features

- **ELF Support**: Directly load and simulate AArch64 ELF executables
- **Out-of-Order Execution**: Models instruction window, dependency tracking, issue/commit bandwidth
- **Memory Subsystem**: Detailed L1/L2 cache modeling with configurable parameters
- **Konata Visualization**: Pipeline stage visualization compatible with Konata format
- **TopDown Analysis**: Intel TopDown methodology for performance bottleneck identification

## Quick Start

### Usage

```bash
cpu_emulator <elf_file> [max_instructions] [output.json]
```

| Argument | Description | Default |
|----------|-------------|---------|
| `elf_file` | Path to AArch64 ELF executable | (required) |
| `max_instructions` | Maximum instructions to simulate | 10000 |
| `output.json` | Output Konata JSON path | `<elf_name>.json` |

### macOS / Linux

```bash
# Clone and build
git clone https://github.com/diqianli/aieda.git
cd aieda
cargo build --release --features visualization

# Run simulation on ELF file
./target/release/examples/cpu_emulator test_programs/fibonacci.elf 50000

# View results
open fibonacci_report.html
# or start HTTP server:
python3 -m http.server 8080
```

### Windows

```cmd
# Build
cargo build --release --features visualization

# Run simulation
target\release\examples\cpu_emulator.exe test_programs\fibonacci.elf 50000

# View results
start fibonacci_report.html
```

## Output Files

After running, three files are generated:

| File | Description |
|------|-------------|
| `<name>.json` | Konata pipeline visualization data |
| `<name>_topdown.json` | TopDown performance analysis |
| `<name>_report.html` | Interactive HTML report |

## TopDown Analysis

The HTML report includes:
- **Summary**: IPC, total cycles, instructions
- **TopDown Metrics**: Retiring, Frontend Bound, Backend Bound, Bad Speculation
- **Pipeline Configuration**: Issue width, commit width, window size

## Prerequisites

- **Rust 1.70+** - Install via [rustup](https://rustup.rs/)
- **Python 3** - For viewing HTML report (optional)

## Examples

```bash
# Basic usage
cpu_emulator program.elf

# Limit to 100K instructions
cpu_emulator program.elf 100000

# Custom output path
cpu_emulator program.elf 50000 results/analysis.json

# Cross-compile ELF (on ARM macOS)
aarch64-linux-gnu-gcc -static -o program.elf program.c
cpu_emulator program.elf
```

## Project Structure

```
arm_cpu_emulator/
├── examples/
│   └── cpu_emulator.rs      # Main executable
├── src/
│   ├── cpu.rs               # CPU emulator
│   ├── elf/                 # ELF loader and decoder
│   ├── ooo/                 # Out-of-order execution
│   └── visualization/       # Konata format support
├── test_programs/           # Sample ELF files
└── .github/workflows/       # CI/CD for releases
```

## Documentation

- [SETUP.md](SETUP.md) - Complete setup and configuration guide
- [README_WINDOWS.txt](README_WINDOWS.txt) - Windows-specific instructions

## License

MIT License
