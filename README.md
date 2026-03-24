# ARM CPU Emulator

ARMv8-A (AArch64) CPU emulator with out-of-order execution simulation and Konata-compatible pipeline visualization.

## Features

- **Out-of-Order Execution**: Models instruction window, dependency tracking, issue/commit bandwidth
- **Memory Subsystem**: Detailed L1/L2 cache modeling with configurable parameters
- **Konata Visualization**: Pipeline stage visualization compatible with Konata format
- **TopDown Analysis**: Intel TopDown methodology for performance bottleneck identification
- **Hotspot Detection**: Identify performance-critical code regions

## Quick Start

### macOS / Linux

```bash
# Clone repository
git clone https://github.com/diqianli/aieda.git
cd aieda

# Build with visualization feature
cargo build --features visualization --release

# Run simulation (100K instructions)
cargo run --features visualization --example generate_konata 100000

# Start visualization server
cd visualization/static && python3 -m http.server 8080

# Open browser:
# - Pipeline: http://localhost:8080/index_static.html
# - TopDown: http://localhost:8080/konata_data_report.html
```

### Windows

```cmd
# Clone repository
git clone https://github.com/diqianli/aieda.git
cd aieda

# Method 1: Run batch script (recommended)
build_windows.bat

# Method 2: Run PowerShell script
powershell -ExecutionPolicy Bypass -File run_simulation.ps1

# Start visualization server
cd visualization\static
python -m http.server 8080

# Open browser:
# - TopDown Report: http://localhost:8080/konata_data_report.html
# - Pipeline View: http://localhost:8080/index_static.html
```

## Prerequisites

### macOS / Linux
- Rust 1.70+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Python 3 (for visualization server)

### Windows
- Rust 1.70+ (download from https://win.rustup.rs/x86_64)
- Visual Studio Build Tools (C++ development)
- Python 3 (for visualization server)

## Output Files

After running the simulation, these files are generated in `visualization/static/`:

| File | Description |
|------|-------------|
| `konata_data.json` | Pipeline visualization data (large file) |
| `konata_data_topdown.json` | TopDown analysis JSON data |
| `konata_data_report.html` | Interactive HTML report with charts |

## Visualization

### TopDown Analysis Report

Open `konata_data_report.html` to see:
- **Summary Metrics**: IPC, total cycles, instructions
- **TopDown Level 1**: Retiring, Frontend Bound, Backend Bound, Bad Speculation
- **Instruction Mix**: ALU, Memory, Branch, SIMD breakdown
- **Pipeline Stage Utilization**: Chart showing each stage's usage
- **Cycle Distribution**: Full issue, partial issue, stall cycles
- **Hotspots**: Top 20 PC ranges by cycle count

### Konata Pipeline View

Open `index_static.html` to see:
- Visual pipeline stage timeline
- Per-instruction stage timing
- Dependency arrows between instructions
- Cycle-by-cycle execution view

## Sample Output

```
=== Konata Data Generator ===
Instructions: 100000
Time: 377.15s
Total cycles: 100001
Instructions committed: 100000
IPC: 1.00

--- Exporting Konata Data ---
Tracked 100000 instructions
Generated 100000 operations

--- Generating TopDown Analysis ---
TopDown Analysis Generated
Exported 100000 operations to konata_data.json
TopDown report exported to konata_data_topdown.json
HTML report exported to konata_data_report.html
```

## Project Structure

```
arm_cpu_emulator/
├── src/                    # Source code
│   ├── cpu.rs              # CPU emulator
│   ├── ooo/                # Out-of-order execution
│   ├── memory/             # Memory subsystem
│   ├── analysis/           # TopDown analysis
│   └── visualization/      # Konata format support
├── examples/               # Example programs
│   └── generate_konata.rs  # Main simulation + export
├── visualization/static/   # Web visualization files
├── build_windows.bat       # Windows build script
├── run_simulation.ps1      # PowerShell build script
├── README_WINDOWS.txt      # Windows detailed instructions
├── SETUP.md                # Complete setup guide
└── README.md               # This file
```

## Documentation

- [SETUP.md](SETUP.md) - Complete setup and configuration guide
- [README_WINDOWS.txt](README_WINDOWS.txt) - Windows-specific instructions

## License

MIT License

---

# AI Agent 在 IC 设计流程中的应用研究

基于微信公众号文章《AI辅助RTL代码生成工具推荐》的深度研究，整理成可视化静态网页。

## 项目结构

```
ai-ic-design-research/
├── index.html          # 总览对比页面
├── cadence.html        # Cadence ChipStack AI 详解
├── siemens.html        # Siemens Questa One 详解
├── synopsys.html       # Synopsys.ai 详解
├── xinhua.html         # 芯华章 ChatDV 详解
├── s2c.html            # 思尔芯 详解
├── opensource.html     # 开源/研究工具汇总
├── styles.css          # 样式文件
├── assets/             # 图片资源目录
└── README.md           # 本文件
```

## 本地预览

直接在浏览器中打开 `index.html` 文件即可预览。

或使用本地服务器：

```bash
# Python 3
python -m http.server 8000

# Node.js
npx serve .
```

## 部署到 GitHub Pages

1. 创建 GitHub 仓库
2. 上传所有文件
3. 在仓库设置中启用 GitHub Pages
4. 选择分支和根目录作为源

## 研究对象

### 国际 EDA 巨头
- **Cadence** - ChipStack AI Super Agent
- **Siemens** - Questa One Agentic Toolkit
- **Synopsys** - Synopsys.ai GenAI

### 国产工具
- **芯华章** - ChatDV + GalaxAI
- **思尔芯 (S2C)** - 原型验证工具

### 开源/研究工具
- **ACE-RTL** - Agentic Context Evolution
- **Saarthi** - 全自主形式验证代理
- **VerilogEval** - RTL 生成质量基准

## 信息来源

- 各公司官方发布
- 学术论文（arXiv）
- 行业媒体报道

## 更新日期

2025年3月
