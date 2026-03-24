# ARM CPU Emulator - Windows 构建说明

## 系统要求

- Windows 10/11 64位
- 至少 4GB RAM
- 至少 2GB 磁盘空间

## 安装步骤

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

或直接下载 ZIP 包并解压。

### 3. 构建项目

双击运行 `build_windows.bat`，或在命令行中执行：

```cmd
build_windows.bat
```

### 4. 查看结果

构建完成后，输出文件位于 `output\` 目录：
- `konata_data.json` - 流水线可视化数据
- `konata_data_topdown.json` - TopDown 分析数据
- `konata_data_report.html` - HTML 可视化报告

### 5. 启动可视化服务

```cmd
cd visualization\static
python -m http.server 8080
```

然后在浏览器中打开：
- **TopDown 报告**: http://localhost:8080/konata_data_report.html
- **流水线视图**: http://localhost:8080/index_static.html

---

## 手动构建步骤

如果自动脚本失败，可以手动执行以下命令：

```cmd
# 1. 构建项目
cargo build --release --features visualization

# 2. 运行模拟（100K 指令）
cargo run --release --features visualization --example generate_konata 100000

# 3. 启动可视化
cd visualization\static
python -m http.server 8080
```

---

## 常见问题

### Q: 提示 "cargo not found"
**A:** 重新打开命令行窗口，或手动添加 Rust 到 PATH：
```cmd
set PATH=%USERPROFILE%\.cargo\bin;%PATH%
```

### Q: 编译报错 "linker not found"
**A:** 安装 Visual Studio Build Tools：
```
https://visualstudio.microsoft.com/visual-cpp-build-tools/
```
选择 "Desktop development with C++"

### Q: Python http.server 不工作
**A:** 确保安装了 Python，或使用 Node.js：
```cmd
npx http-server -p 8080
```

### Q: 浏览器显示空白页面
**A:**
1. 确认 `konata_data.json` 文件存在
2. 按 F12 检查浏览器控制台错误
3. 尝试硬刷新（Ctrl+Shift+R）

---

## 项目结构

```
aieda/
├── Cargo.toml              # 项目配置
├── src/                    # 源代码
│   ├── cpu.rs              # CPU 模拟器
│   ├── ooo/                # 乱序执行引擎
│   ├── memory/             # 内存子系统
│   └── visualization/      # 可视化支持
├── examples/               # 示例程序
├── visualization/static/   # Web 可视化文件
├── build_windows.bat       # Windows 构建脚本
└── README_WINDOWS.txt      # 本文件
```

---

## 输出示例

```
=== Konata Data Generator ===
Instructions: 100000
Time: 120.5s
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

---

## 技术支持

- GitHub Issues: https://github.com/diqianli/aieda/issues
- 详细文档: 见 SETUP.md
