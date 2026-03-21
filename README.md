# ARM CPU Emulator

ARMv8-A (AArch64) CPU emulator with out-of-order execution simulation and Konata-compatible pipeline visualization.

## Features

- **Out-of-Order Execution**: Models instruction window, dependency tracking, issue/commit bandwidth
- **Memory Subsystem**: Detailed L1/L2 cache modeling with configurable parameters
- **Konata Visualization**: Pipeline stage visualization compatible with Konata format
- **TopDown Analysis**: Intel TopDown methodology for performance bottleneck identification
- **Hotspot Detection**: Identify performance-critical code regions

## Quick Start

```bash
# Clone repository
git clone git@github.com:diqianli/aieda.git
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

## Documentation

See [SETUP.md](SETUP.md) for complete setup and usage instructions.

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
├── visualization/static/   # Web visualization files
└── SETUP.md               # Detailed setup guide
```

## Sample Output

```
=== Konata Data Generator ===
Instructions: 100000
Time: 377.15s
Total cycles: 100001
Instructions committed: 100000
IPC: 1.00

TopDown Analysis:
- Retiring: 16.7%
- Frontend Bound: 10.7%
- Backend Bound: 16.0%
- Bad Speculation: 56.7%
```

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
