# ARM CPU Emulator - Windows 绿色版使用说明

## 快速开始

### 运行仿真

```cmd
cpu_emulator.exe <elf_file> [max_instructions] [output.json]
```

| 参数 | 说明 | 默认值 |
|------|------|--------|
| elf_file | AArch64 ELF 可执行文件路径 | (必填) |
| max_instructions | 最大模拟指令数 | 10000 |
| output.json | 输出 JSON 路径 | <elf名>.json |

### 示例

```cmd
# 运行示例程序
cpu_emulator.exe sample.elf

# 限制 10 万条指令
cpu_emulator.exe sample.elf 100000

# 指定输出路径
cpu_emulator.exe sample.elf 50000 output\analysis.json
```

## 输出文件

运行后生成三个文件：

| 文件 | 说明 |
|------|------|
| `<name>.json` | Konata 流水线可视化数据 |
| `<name>_topdown.json` | TopDown 性能分析数据 |
| `<name>_report.html` | HTML 可视化报告 |

## 查看结果

### 方法一：直接打开 HTML

```cmd
start sample_report.html
```

### 方法二：启动 HTTP 服务器

```cmd
python -m http.server 8080
# 浏览器打开 http://localhost:8080/sample_report.html
```

## 文件说明

| 文件 | 说明 |
|------|------|
| `cpu_emulator.exe` | 主程序 - ARM CPU 仿真器 |
| `sample.elf` | 示例 ELF 程序 |
| `run.bat` | 一键运行脚本 |
| `README_WINDOWS.txt` | 本说明文件 |

## 系统要求

- Windows 10/11 64位
- 4GB+ 内存
- 无需安装任何依赖

## 常见问题

### Q: 运行时闪退
A: 请在命令行中运行，查看错误信息：
```cmd
cpu_emulator.exe sample.elf
```

### Q: ELF 文件格式错误
A: 确保 ELF 是 AArch64 (ARM64) 架构：
```cmd
# Linux 交叉编译
aarch64-linux-gnu-gcc -static -o program.elf program.c
```

### Q: 内存不足
A: 减少指令数量：
```cmd
cpu_emulator.exe sample.elf 1000
```

## TopDown 分析说明

| 指标 | 说明 | 理想值 |
|------|------|--------|
| Retiring | 有效完成的工作 | >50% |
| Frontend Bound | 取指/解码瓶颈 | <10% |
| Backend Bound | 执行/内存瓶颈 | <20% |
| Bad Speculation | 分支预测浪费 | <10% |

## 技术支持

- GitHub: https://github.com/diqianli/aieda
- 问题反馈: https://github.com/diqianli/aieda/issues
