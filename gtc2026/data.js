// GTC 2026 Leaks Data
const leaksData = {
  products: [
    {
      id: 'feynman',
      name: 'Feynman 架构',
      year: '2028',
      icon: '🚀',
      description: '下一代GPU架构，采用先进制程和3D堆叠技术',
      specs: [
        { attr: '制程', info: '1nm/1.6nm，TSMC A16 节点', credibility: 75 },
        { attr: '内存', info: 'HBM4/HBM4e', credibility: 80 },
        { attr: '特性', info: '3D堆叠SRAM chiplets，PowerVia背面供电', credibility: 70 },
        { attr: '功耗', info: '可能达到 5000W', credibility: 50 },
        { attr: '性能', info: '可能突破 200 petaflops', credibility: 45 },
        { attr: '合作', info: '部分I/O die可能由Intel代工', credibility: 55 },
        { attr: '时间线', info: '2028年发布', credibility: 85 }
      ],
      sources: ['Igor\'s Lab', 'SemiAnalysis', '中文科技媒体']
    },
    {
      id: 'rubin',
      name: 'Vera Rubin GPU',
      year: '2026 H2',
      icon: '💎',
      description: '2026年下半年发布的旗舰数据中心GPU',
      specs: [
        { attr: 'Vera CPU', info: '88核/176线程，自定义ARM "Olympus"架构', credibility: 95 },
        { attr: '内存', info: '288GB HBM4 per GPU', credibility: 95 },
        { attr: '带宽', info: '22 TB/s 内存带宽', credibility: 90 },
        { attr: '性能', info: '50 PFLOPS FP4', credibility: 90 },
        { attr: '对比Blackwell', info: '训练3.5x，推理5x', credibility: 85 },
        { attr: 'NVLink', info: 'NVLink 6，3600 GB/s', credibility: 85 },
        { attr: '设计', info: '双chiplet GPU设计', credibility: 90 },
        { attr: '时间线', info: '2026年下半年，Q3生产', credibility: 90 }
      ],
      sources: ['NVIDIA官方', 'Tom\'s Hardware', 'Wccftech', 'Reddit']
    },
    {
      id: 'rubin-ultra',
      name: 'Rubin Ultra',
      year: '2027 H2',
      icon: '⚡',
      description: 'Rubin的终极版本，瞄准超大规模AI训练',
      specs: [
        { attr: 'GPU', info: '4个reticle-sized GPU', credibility: 85 },
        { attr: '内存', info: '1TB HBM4e', credibility: 80 },
        { attr: '性能', info: '100 petaflops FP4', credibility: 75 },
        { attr: '机架性能', info: '15 exaflops FP4推理, 5 exaflops FP8训练', credibility: 70 },
        { attr: '提升', info: '相比Rubin 14x性能提升', credibility: 70 },
        { attr: '时间线', info: '2027年下半年', credibility: 80 }
      ],
      sources: ['VideoCardz', 'Ars Technica', 'The Verge']
    },
    {
      id: 'n1x',
      name: 'N1/N1X ARM 笔记本芯片',
      year: '2026 H1',
      icon: '💻',
      description: 'NVIDIA进军Windows on ARM游戏笔记本市场',
      specs: [
        { attr: 'CPU', info: '20核 (2x 10核集群)', credibility: 75 },
        { attr: 'GPU', info: '6144 CUDA核心 (~RTX 5070级别)', credibility: 70 },
        { attr: '合作伙伴', info: 'Dell, Lenovo', credibility: 80 },
        { attr: '基础', info: 'GB10 Superchip (DGX Spark)', credibility: 70 },
        { attr: '目标', info: 'Windows on ARM 游戏笔记本', credibility: 75 },
        { attr: '时间线', info: '2026年上半年', credibility: 65 }
      ],
      sources: ['Tom\'s Hardware', 'Wall Street Journal', 'Notebookcheck', 'Reddit']
    },
    {
      id: 'blackwell-ultra',
      name: 'Blackwell Ultra',
      year: '2025 H2',
      icon: '🔥',
      description: 'Blackwell架构的增强版本',
      specs: [
        { attr: '型号', info: 'B300系列, GB300 NVL72', credibility: 95 },
        { attr: '性能', info: '50x throughput per megawatt', credibility: 85 },
        { attr: '成本', info: '35x lower cost per token', credibility: 80 },
        { attr: '时间线', info: '2025年下半年发布', credibility: 95 }
      ],
      sources: ['NVIDIA官方', 'TechPowerUp', 'SemiAnalysis']
    },
    {
      id: 'rtx60',
      name: 'RTX 60 系列',
      year: '2027',
      icon: '🎮',
      description: '下一代消费级显卡，Rubin架构消费版',
      specs: [
        { attr: '架构', info: 'Rubin架构消费版', credibility: 65 },
        { attr: 'RTX 6090', info: '2027 Q1 或 H2', credibility: 55 },
        { attr: '芯片代号', info: 'GR20X', credibility: 60 },
        { attr: '性能提升', info: '10-30% vs RTX 5090', credibility: 50 }
      ],
      sources: ['Kopite7kimi', 'Reddit', 'Overclock.net']
    },
    {
      id: 'dlss45',
      name: 'DLSS 4.5',
      year: 'GDC 2026',
      icon: '✨',
      description: '下一代AI超分辨率技术',
      specs: [
        { attr: '特性', info: 'Dynamic Multi Frame Generation', credibility: 100 },
        { attr: '模式', info: '6X Multi Frame Generation', credibility: 100 },
        { attr: '游戏', info: '20+ 新支持游戏', credibility: 95 },
        { attr: '时间线', info: '2026年3月31日 Beta', credibility: 100 }
      ],
      sources: ['NVIDIA官方']
    },
    {
      id: 'cpo',
      name: 'CPO 光互连',
      year: '2026 Q2',
      icon: '💡',
      description: 'Co-Packaged Optics 光互连技术',
      specs: [
        { attr: '产品', info: 'Spectrum-X Photonics 交换机', credibility: 90 },
        { attr: '速率', info: '3.2 Tb/s 光引擎', credibility: 85 },
        { attr: '目标', info: '扩展AI工厂到百万GPU', credibility: 80 },
        { attr: '时间线', info: '2026 Q2 量产', credibility: 75 }
      ],
      sources: ['NVIDIA官方', 'TrendForce', 'LinkedIn']
    }
  ],
  rumors: [
    { info: 'NVIDIA x86 CPU (与Intel合作)', credibility: 30, note: 'Intel已否认' },
    { info: 'Groq技术整合，新推理芯片', credibility: 60, note: '未确认' },
    { info: 'LPX机架从64扩展到256 LPU', credibility: 55, note: '推测' },
    { info: 'NemoClaw AI Agent开源平台', credibility: 70, note: '社区讨论' },
    { info: '人形机器人更新', credibility: 75, note: 'Project GR00T相关' },
    { info: '"震惊世界"的神秘芯片', credibility: 20, note: '纯猜测' }
  ],
  eventInfo: {
    dates: '2026年3月16-19日',
    location: 'San Jose, CA',
    keynote: 'Jensen Huang, 3月16日',
    themes: ['Physical AI', 'Agentic AI', 'Inference', 'AI Factories']
  }
};

// Word cloud data with weights
const wordCloudData = [
  // 高权重 (核心产品)
  ['Rubin', 100],
  ['Vera', 90],
  ['HBM4', 85],
  ['NVLink', 80],
  ['Feynman', 75],
  ['AI Factory', 70],
  ['Inference', 70],
  ['50 PFLOPS', 68],
  ['288GB', 65],
  ['ARM', 65],

  // 中权重 (技术特性)
  ['88-core', 60],
  ['CPO', 55],
  ['Photonics', 50],
  ['DLSS 4.5', 50],
  ['Path Tracing', 45],
  ['3D Stacking', 45],
  ['PowerVia', 42],
  ['TSMC A16', 40],
  ['N1X', 40],
  ['Laptop', 40],
  ['Blackwell Ultra', 38],
  ['RTX 60', 35],
  ['RTX 6090', 35],
  ['Groq', 35],
  ['B300', 32],

  // 低权重 (传闻/次要)
  ['x86', 30],
  ['5000W', 25],
  ['1nm', 28],
  ['200 PFLOPS', 22],
  ['Windows ARM', 20],
  ['DGX Spark', 18],
  ['Olympus', 15]
];

// Timeline data
const timelineData = [
  { year: '2025 H2', event: 'Blackwell Ultra', color: '#76b900' },
  { year: '2026 Q1', event: 'DLSS 4.5 Beta', color: '#76b900' },
  { year: '2026 H1', event: 'N1/N1X 笔记本芯片', color: '#ffb800' },
  { year: '2026 Q2', event: 'CPO 光互连量产', color: '#76b900' },
  { year: '2026 H2', event: 'Vera Rubin GPU', color: '#76b900' },
  { year: '2027 H1', event: 'RTX 60 系列?', color: '#ff6b6b' },
  { year: '2027 H2', event: 'Rubin Ultra', color: '#ffb800' },
  { year: '2028', event: 'Feynman 架构', color: '#ffb800' }
];
