# Agentic AI CPU 设计空间可视化

基于 NVIDIA Vera CPU 和 Arm AGI CPU 的公开信息，创建的交互式可视化内容，展示 Agentic AI 场景下四个维度（系统、SoC、CPU、软硬件协同）的设计空间。

## 📁 项目文件

### 核心文件

| 文件 | 说明 |
|------|------|
| `diagram_generator.html` | 示意图生成器 - 浏览器中打开，生成4个维度的可视化配图 |
| `generate_agi_cpu_ppt.py` | PPT生成脚本 - 生成华为风格的4页PowerPoint |
| `Agentic_AI_CPU_Design_Space.pptx` | 生成的PPT文件 (4页，华为红配色) |

## 🎨 四个设计维度

### Level 1: 系统维度设计空间
- **CPU-GPU配比模式**: 训练优化(1:2)、均衡型(1:1)、GPU为主(1:4)
- **机架功率密度**: 标准(20kW)、高密度(36kW风冷)、液冷(60kW+)
- **分层系统架构**: 分离三层、统一调度、混合模式
- **节点间互联拓扑**: NVLink-C2C (1.8TB/s)、InfiniBand、RoCEv2
- **系统级内存架构**: 本地内存、CXL 3.0池化、分解式内存
- **部署模式**: 传统数据中心、AI工厂、边缘部署

### Level 2: SoC维度设计空间
- **核心架构选择**: 公版核心(Arm Neoverse)、自研核心(NVIDIA Olympus)、混合架构
- **缓存层次结构**: 能效优先、大缓存配置(162MB L3)、自适应缓存
- **Die架构策略**: 单片Monolithic(零NUMA)、Chiplet(灵活扩展)、混合架构
- **内存子系统**: DDR5 DIMM、SOCAMM(1.5TB可更换)、HBM高带宽
- **I/O配置**: 标准配置、高密度I/O(96条PCIe Gen6)、定制I/O
- **工艺节点**: 5nm成熟工艺、3nm领先工艺、2nm前沿工艺

### Level 3: CPU维度设计空间
- **多线程策略**: 无SMT、传统SMT(时间切片)、空间多线程(物理隔离)
- **AI指令集扩展**: BF16/INT8、FP8原生支持、定制扩展指令
- **分支预测器**: 静态预测、动态预测(传统)、神经预测器(ML)
- **指令前端宽度**: 6-wide(能效)、8-wide(均衡)、10-wide(高性能)
- **片上专用加速器**: RDMA引擎、加密引擎、压缩引擎、全加速套件
- **频率与功耗策略**: 能效优先、平衡策略、激进boost(延迟敏感)

### Level 4: 软硬件协同设计空间
- **CPU端瓶颈优化**: 基础优化、激进优化(73%收益)、软硬协同
- **NUMA感知调度**: 手动绑定(bindpcie)、自动调度、混合模式
- **软件栈优化**: 驱动层优化、框架层优化、全栈优化(Python→JIT)
- **生态兼容性策略**: ARM原生、混合部署、二进制翻译
- **性能可观测性**: 分离工具、统一视图(CPU-GPU对齐)、AI分析
- **运行时动态调优**: 静态配置、动态调优(SMT/频率)、预测性调优

## 🚀 使用方式

### 生成示意图
1. 在浏览器中打开 `diagram_generator.html`
2. 查看四个维度的可视化示意图
3. 点击 "📥 下载所有示意图" 按钮下载PNG图片
4. 将图片插入PPT或其他文档

### 生成PPT
```bash
# 安装依赖
pip install python-pptx

# 运行生成脚本
python generate_agi_cpu_ppt.py

# 输出文件: Agentic_AI_CPU_Design_Space.pptx
```

## 🎨 设计风格

### 华为红色配色方案
- **主色**: 华为红 RGB(200, 16, 46) / #C8102E
- **深红**: RGB(139, 0, 0) / #8B0000
- **浅红**: RGB(232, 93, 117) / #E85D75
- **强调色**: 金色 RGB(245, 166, 35) / #F5A623

### PPT布局
- 红色标题头 + 白色内容区
- 2×3网格卡片布局
- 底部金色边框技术洞察框
- Level 1-4 金色徽章

## 📊 技术洞察

### 阿姆达尔定律在AI系统中的应用
> NVIDIA明确指出"系统性能越来越受制于agentic循环中CPU端串行任务的限制"。DeepSeek-V3在GB200上的优化显示，73%的约349 TFLOPS性能提升来自解决CPU overhead。

### CPU瓶颈的具体表现
在GB200上，Grace的kernel launch开销约为x86 Xeon的2倍。Blackwell GPU算力提升2.5倍后，CPU端"掩盖"GPU kernel间隙的时间急剧缩短，使瓶颈更加明显。

### NUMA绑定的收益
DeepSeek-V3通过bindpcie将进程绑定到本地GPU，获得从691到762 TFLOPS的提升（+70.6 TFLOPS）。

## 📚 参考资料

- NVIDIA Vera CPU Architecture
- Arm AGI CPU Specifications
- DeepSeek-V3 Technical Report
- GB200 System Optimization Guidelines

## 📝 许可证

本项目内容仅供学习交流使用。
