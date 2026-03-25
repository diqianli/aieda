# ARM CPU Emulator - Windows 绿色版使用说明

## 快速开始

### 方法一：直接运行（推荐）

1. 双击 `run.bat` 运行模拟
2. 等待模拟完成（约1-2分钟）
3. 查看 `output` 目录中的结果文件

### 方法二：命令行运行

```cmd
# 运行 10万条指令的模拟
generate_konata.exe 100000 static\konata_data.json

# 查看结果
cd static
python -m http.server 8080
```

然后打开浏览器访问：
- **TopDown 报告**: http://localhost:8080/konata_data_report.html
- **流水线视图**: http://localhost:8080/index_static.html

## 文件说明

| 文件 | 说明 |
|------|------|
| `generate_konata.exe` | 主程序 - 生成 Konata 可视化数据 |
| `run.bat` | 一键运行脚本 |
| `static/` | 可视化网页文件目录 |
| `README_WINDOWS.txt` | 本说明文件 |

## 输出文件

运行后在 `static/` 目录生成：

| 文件 | 说明 |
|------|------|
| `konata_data.json` | 流水线可视化数据（较大） |
| `konata_data_topdown.json` | TopDown 性能分析数据 |
| `konata_data_report.html` | HTML 可视化报告 |

## 参数说明

```cmd
generate_konata.exe [指令数量] [输出路径]

示例：
generate_konata.exe 10000 output.json        # 1万条指令
generate_konata.exe 100000 output.json        # 10万条指令
generate_konata.exe 1000000 output.json       # 100万条指令（需要较长时间）
```

## 系统要求

- Windows 10/11 64位
- 4GB+ 内存（100万条指令需要 8GB+）
- 无需安装任何依赖

## 常见问题

### Q: 运行时闪退
A: 请在命令行中运行，查看错误信息：
```cmd
cd 解压目录
generate_konata.exe 10000 test.json
```

### Q: Python http.server 不工作
A: 确保安装了 Python，或使用其他方式：
```cmd
# 使用 Node.js
npx http-server -p 8080

# 或直接双击打开 konata_data_report.html（部分功能可能受限）
```

### Q: 内存不足
A: 减少指令数量：
```cmd
generate_konata.exe 10000 output.json
```

## TopDown 分析说明

TopDown 报告包含以下指标：

| 指标 | 说明 | 理想值 |
|------|------|--------|
| Retiring | 有效完成的工作 | >50% |
| Frontend Bound | 取指/解码瓶颈 | <10% |
| Backend Bound | 执行/内存瓶颈 | <20% |
| Bad Speculation | 分支预测浪费 | <10% |

---

## 从源码构建（可选）

如果需要从源码构建：

### 1. 安装 Rust

下载并运行 Rust 安装程序：
```
https://win.rustup.rs/x86_64
```

或使用命令行：
```powershell
winget install Rustlang.Rustup
```

安装完成后，重新打开命令行窗口验证：
```cmd
rustc --version
cargo --version
```

### 2. 克隆项目

```cmd
git clone https://github.com/diqianli/aieda.git
cd aieda
```

### 3. 构建项目

双击运行 `build_windows.bat`，或在命令行中执行：

```cmd
build_windows.bat
```

### 4. 手动构建

```cmd
cargo build --release --features visualization
cargo run --release --features visualization --example generate_konata 100000
```

---

## 技术支持

- GitHub: https://github.com/diqianli/aieda
- 问题反馈: https://github.com/diqianli/aieda/issues
- 详细文档: 见 SETUP.md
