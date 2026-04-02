/**
 * ARM Lock Mechanisms - Barrier Deep Analysis Page
 * Pthread Barrier 7 Real-World Scenarios
 */

const BarrierPage = {
  currentScenario: null,
  animationId: null,

  /**
   * Initialize the barrier page
   */
  init: function() {
    this.renderScenarioTabs();
    this.renderComparisonTable();
    this.selectScenario('mapreduce-spark');
    this.initTopologyCanvas();
  },

  /**
   * Render scenario selector tabs
   */
  renderScenarioTabs: function() {
    const container = document.getElementById('scenario-tabs');
    if (!container) return;

    const html = LockData.barrierScenarios.map(scenario => `
      <button class="scenario-tab ${scenario.id === this.currentScenario ? 'active' : ''}"
              data-scenario-id="${scenario.id}"
              onclick="BarrierPage.selectScenario('${scenario.id}')">
        <span class="tab-icon">${scenario.icon}</span>
        <span class="tab-label">${scenario.name}</span>
      </button>
    `).join('');

    container.innerHTML = html;
  },

  /**
   * Select a scenario and render its content
   */
  selectScenario: function(scenarioId) {
    this.currentScenario = scenarioId;

    // Update tab active states
    document.querySelectorAll('.scenario-tab').forEach(tab => {
      if (tab.dataset.scenarioId === scenarioId) {
        tab.classList.add('active');
      } else {
        tab.classList.remove('active');
      }
    });

    // Get scenario data
    const scenario = LockData.barrierScenarios.find(s => s.id === scenarioId);
    if (!scenario) return;

    // Render scenario content
    this.renderScenarioContent(scenario);
  },

  /**
   * Render scenario content
   */
  renderScenarioContent: function(scenario) {
    const container = document.getElementById('scenario-content');
    if (!container) return;

    const html = `
      <div class="scenario-detail">
        <!-- Scenario Header -->
        <div class="scenario-header">
          <div class="scenario-title-large">
            <span class="scenario-icon-large" style="color: ${scenario.color}">${scenario.icon}</span>
            <div>
              <h3>${scenario.name}</h3>
              <p class="scenario-category">${scenario.category}</p>
            </div>
          </div>
          <div class="real-software">
            <strong>真实软件：</strong>
            ${scenario.realSoftware.map(sw => `<span class="software-tag">${sw}</span>`).join('')}
          </div>
        </div>

        <!-- Description -->
        <div class="scenario-description">
          <p>${scenario.description}</p>
        </div>

        <!-- Barrier Role -->
        <div class="barrier-role-section">
          <h4>Barrier 在此场景中的角色</h4>
          <p>${scenario.barrierRole}</p>
        </div>

        <!-- Phases Flow -->
        <div class="phases-section">
          <h4>执行阶段流程</h4>
          <div class="phases-flow">
            ${this.renderPhasesFlow(scenario)}
          </div>
        </div>

        <!-- Hardware Analysis Grid -->
        <div class="hardware-analysis-grid">
          <div class="hardware-analysis-card near">
            <h5><span style="color: #00d1ff">◉</span> Near-Atomic 影响</h5>
            <p>${scenario.hardwareAnalysis.nearAtomicImpact}</p>
          </div>
          <div class="hardware-analysis-card far">
            <h5><span style="color: #ff9500">◎</span> Far-Atomic 影响</h5>
            <p>${scenario.hardwareAnalysis.farAtomicImpact}</p>
          </div>
        </div>

        <!-- Cache Topology Analysis -->
        <div class="cache-topology-analysis">
          <h4>缓存拓扑性能分析</h4>
          <table class="topology-perf-table">
            <thead>
              <tr>
                <th>拓扑配置</th>
                <th>Barrier 延迟</th>
                <th>吞吐量</th>
                <th>推荐策略</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>同集群</td>
                <td>${scenario.hardwareAnalysis.cacheTopology.sameCluster}</td>
                <td>高</td>
                <td>Near-Atomic 优先</td>
              </tr>
              <tr>
                <td>跨集群</td>
                <td>${scenario.hardwareAnalysis.cacheTopology.crossCluster}</td>
                <td>中</td>
                <td>混合模式</td>
              </tr>
              <tr>
                <td>跨 NUMA</td>
                <td>${scenario.hardwareAnalysis.cacheTopology.crossNuma}</td>
                <td>低</td>
                <td>Far-Atomic 优化</td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- ARM Performance Table -->
        <div class="arm-performance-section">
          <h4>ARM Neoverse 性能表现</h4>
          <table class="arm-perf-table">
            <thead>
              <tr>
                <th>处理器</th>
                <th>核心数</th>
                <th>Barrier 延迟</th>
                <th>吞吐量</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>Graviton2 (ARM Neoverse N1)</td>
                <td>${scenario.armPerformance.graviton2.cores}</td>
                <td>${scenario.armPerformance.graviton2.barrierLatency}</td>
                <td>${scenario.armPerformance.graviton2.throughput}</td>
              </tr>
              <tr>
                <td>Graviton3 (ARM Neoverse V1)</td>
                <td>${scenario.armPerformance.graviton3.cores}</td>
                <td>${scenario.armPerformance.graviton3.barrierLatency}</td>
                <td>${scenario.armPerformance.graviton3.throughput}</td>
              </tr>
              <tr>
                <td>Graviton4 (ARM Neoverse V2)</td>
                <td>${scenario.armPerformance.graviton4.cores}</td>
                <td>${scenario.armPerformance.graviton4.barrierLatency}</td>
                <td>${scenario.armPerformance.graviton4.throughput}</td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- Concurrency Analysis Section -->
        ${this.renderConcurrencyAnalysis(scenario)}

        <!-- Code Example -->
        <div class="code-example-section">
          <h4>代码示例</h4>
          <div class="code-block">
            <pre><code>${this.escapeHtml(scenario.codeExample)}</code></pre>
          </div>
        </div>
      </div>
    `;

    container.innerHTML = html;

    // Draw phases flow diagram
    setTimeout(() => {
      this.drawPhasesFlowDiagram(scenario);
    }, 100);
  },

  /**
   * Render phases flow HTML
   */
  renderPhasesFlow: function(scenario) {
    return scenario.phases.map((phase, index) => `
      <div class="phase-item ${phase.barrierType}">
        <div class="phase-number">${index + 1}</div>
        <div class="phase-content">
          <div class="phase-name">${phase.name}</div>
          <div class="phase-desc">${phase.desc}</div>
          ${phase.barrierType === 'explicit' ? '<span class="barrier-indicator">⛏️ Barrier</span>' : ''}
        </div>
        ${index < scenario.phases.length - 1 ? '<div class="phase-arrow">→</div>' : ''}
      </div>
    `).join('');
  },

  /**
   * Draw phases flow diagram on canvas
   */
  drawPhasesFlowDiagram: function(scenario) {
    const canvas = document.getElementById('phases-flow-canvas');
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    const width = canvas.width = canvas.offsetWidth * 2;
    const height = canvas.height = 200;

    ctx.scale(2, 2);
    ctx.clearRect(0, 0, width, height);

    const phases = scenario.phases;
    const boxWidth = 120;
    const boxHeight = 50;
    const gap = 60;
    const startX = 40;
    const y = 50;

    phases.forEach((phase, index) => {
      const x = startX + index * (boxWidth + gap);

      // Draw box
      const isBarrier = phase.barrierType === 'explicit';
      ctx.fillStyle = isBarrier ? 'rgba(255, 149, 0, 0.2)' : 'rgba(42, 113, 255, 0.15)';
      ctx.strokeStyle = isBarrier ? '#ff9500' : '#2a71ff';
      ctx.lineWidth = 2;

      ctx.beginPath();
      ctx.roundRect(x, y, boxWidth, boxHeight, 8);
      ctx.fill();
      ctx.stroke();

      // Draw text
      ctx.fillStyle = 'white';
      ctx.font = '12px system-ui';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(phase.name, x + boxWidth / 2, y + boxHeight / 2);

      // Draw arrow to next phase
      if (index < phases.length - 1) {
        const arrowStartX = x + boxWidth;
        const arrowEndX = x + boxWidth + gap;

        ctx.strokeStyle = 'rgba(255, 255, 255, 0.5)';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(arrowStartX, y + boxHeight / 2);
        ctx.lineTo(arrowEndX, y + boxHeight / 2);
        ctx.stroke();

        // Arrowhead
        ctx.beginPath();
        ctx.moveTo(arrowEndX, y + boxHeight / 2);
        ctx.lineTo(arrowEndX - 8, y + boxHeight / 2 - 5);
        ctx.lineTo(arrowEndX - 8, y + boxHeight / 2 + 5);
        ctx.closePath();
        ctx.fillStyle = 'rgba(255, 255, 255, 0.5)';
        ctx.fill();
      }

      // Draw barrier indicator
      if (isBarrier) {
        ctx.fillStyle = '#ff9500';
        ctx.font = '10px system-ui';
        ctx.fillText('⛏️ Barrier', x + boxWidth / 2, y + boxHeight + 15);
      }
    });
  },

  /**
   * Render comparison table
   */
  renderComparisonTable: function() {
    const tbody = document.getElementById('comparison-tbody');
    if (!tbody) return;

    const html = LockData.barrierScenarios.map(scenario => `
      <tr>
        <td><strong>${scenario.icon} ${scenario.name}</strong></td>
        <td>${this.getBarrierFrequencyLabel(scenario.barrierFrequency)}</td>
        <td><span class="atomic-badge ${scenario.optimalAtomicMode}">${scenario.optimalAtomicMode}</span></td>
        <td style="color: #34c759">${scenario.hardwareAnalysis.cacheTopology.sameCluster}</td>
        <td style="color: #ff9500">${scenario.hardwareAnalysis.cacheTopology.crossCluster}</td>
        <td style="color: #ff008a">${scenario.hardwareAnalysis.cacheTopology.crossNuma}</td>
      </tr>
    `).join('');

    tbody.innerHTML = html;
  },

  /**
   * Get barrier frequency label
   */
  getBarrierFrequencyLabel: function(frequency) {
    const labels = {
      'high': '🔴 高频',
      'medium': '🟡 中频',
      'low': '🟢 低频'
    };
    return labels[frequency] || frequency;
  },

  /**
   * Initialize topology canvas
   */
  initTopologyCanvas: function() {
    this.topologyCanvas = document.getElementById('topology-animation-canvas');
    if (this.topologyCanvas) {
      this.topologyCtx = this.topologyCanvas.getContext('2d');
      this.topologyCanvas.width = this.topologyCanvas.offsetWidth * 2;
      this.topologyCanvas.height = this.topologyCanvas.offsetHeight * 2;
      this.topologyCtx.scale(2, 2);
    }
  },

  /**
   * Run topology animation
   */
  runTopologyAnimation: function() {
    if (this.animationId) {
      cancelAnimationFrame(this.animationId);
    }

    const select = document.getElementById('topology-select');
    const topology = select.value;
    const infoDiv = document.getElementById('topology-info');

    // Update info
    const info = {
      'same-cluster': '同一 CPU 集群：线程间通过 L2/L3 缓存共享域通信。Barrier 计数器在本核 L1 命中时可达 Near-Atomic (~3-5 cycles)，但多核同步产生 Unique→Shared 状态转换，部分操作仍需 CHI 互连。',
      'cross-cluster': '跨集群：线程分布在不同集群但同一 Socket，L1 未命中时需通过 CHI 互连到 Home Node，Far-Atomic (~20-100+ cycles) 占主导。混合模式下本地计算仍可 Near-Atomic 优化。',
      'cross-numa': '跨 NUMA 节点：线程分布在不同 Socket，所有 Barrier 操作都通过 CHI 互连层。请求节点保持 Shared 状态，Home Node 处理原子操作，纯 Far-Atomic 模式以减少数据移动。'
    };

    infoDiv.innerHTML = `<p>${info[topology]}</p>`;

    // Clear and start animation
    this.clearTopologyCanvas();
    this.animateBarrierSync(topology);
  },

  /**
   * Clear topology canvas
   */
  clearTopologyCanvas: function() {
    if (!this.topologyCanvas || !this.topologyCtx) return;
    this.topologyCtx.clearRect(0, 0, this.topologyCanvas.width, this.topologyCanvas.height);
  },

  /**
   * Animate barrier synchronization
   */
  animateBarrierSync: function(topology) {
    const ctx = this.topologyCtx;
    const canvas = this.topologyCanvas;
    const width = canvas.offsetWidth;
    const height = canvas.offsetHeight;

    // Define core positions based on topology
    let cores = [];
    if (topology === 'same-cluster') {
      cores = [
        { x: 80, y: 60, id: 0, waiting: false },
        { x: 150, y: 60, id: 1, waiting: false },
        { x: 110, y: 120, id: 2, waiting: false },
        { x: 180, y: 120, id: 3, waiting: false }
      ];
    } else if (topology === 'cross-cluster') {
      cores = [
        { x: 60, y: 60, id: 0, waiting: false },
        { x: 120, y: 60, id: 1, waiting: false },
        { x: 220, y: 60, id: 2, waiting: false },
        { x: 280, y: 60, id: 3, waiting: false }
      ];
    } else { // cross-numa
      cores = [
        { x: 40, y: 60, id: 0, waiting: false },
        { x: 100, y: 60, id: 1, waiting: false },
        { x: 260, y: 60, id: 2, waiting: false },
        { x: 320, y: 60, id: 3, waiting: false }
      ];
    }

    let arrivedCount = 0;
    let lastArrivedTime = 0;
    let phase = 'arriving'; // arriving, waiting, releasing
    let wavePos = 0;

    const draw = (timestamp) => {
      ctx.clearRect(0, 0, width, height);

      // Draw topology background
      this.drawTopologyBackground(ctx, topology, width, height);

      // Draw cores
      cores.forEach(core => {
        // Core circle
        ctx.beginPath();
        ctx.arc(core.x, core.y, 15, 0, Math.PI * 2);
        ctx.fillStyle = core.waiting ? '#ff9500' : '#2a71ff';
        ctx.fill();
        ctx.strokeStyle = 'white';
        ctx.lineWidth = 2;
        ctx.stroke();

        // Core label
        ctx.fillStyle = 'white';
        ctx.font = '10px system-ui';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(`C${core.id}`, core.x, core.y);
      });

      // Animation phases
      if (phase === 'arriving') {
        // Randomly make cores arrive
        if (Math.random() < 0.02 && arrivedCount < cores.length) {
          const waitingCores = cores.filter(c => !c.waiting);
          if (waitingCores.length > 0) {
            const core = waitingCores[Math.floor(Math.random() * waitingCores.length)];
            core.waiting = true;
            arrivedCount++;
            lastArrivedTime = timestamp;
          }
        }

        if (arrivedCount === cores.length) {
          phase = 'releasing';
        }
      } else if (phase === 'releasing') {
        // Release wave animation
        wavePos += 3;

        ctx.beginPath();
        ctx.arc(width / 2, height / 2, wavePos, 0, Math.PI * 2);
        ctx.strokeStyle = 'rgba(52, 199, 89, 0.8)';
        ctx.lineWidth = 3;
        ctx.stroke();

        if (wavePos > Math.max(width, height)) {
          // Reset
          arrivedCount = 0;
          wavePos = 0;
          cores.forEach(c => c.waiting = false);
          phase = 'arriving';
        }
      }

      // Draw status text
      ctx.fillStyle = 'rgba(255, 255, 255, 0.8)';
      ctx.font = '12px system-ui';
      ctx.textAlign = 'center';
      ctx.fillText(
        phase === 'arriving' ? `线程到达中: ${arrivedCount}/${cores.length}` : '释放同步中...',
        width / 2,
        height - 20
      );

      this.animationId = requestAnimationFrame(draw);
    };

    this.animationId = requestAnimationFrame(draw);
  },

  /**
   * Draw topology background
   */
  drawTopologyBackground: function(ctx, topology, width, height) {
    ctx.save();

    if (topology === 'same-cluster') {
      // Draw cluster boundary
      ctx.strokeStyle = 'rgba(0, 209, 255, 0.3)';
      ctx.setLineDash([5, 5]);
      ctx.strokeRect(30, 30, 200, 140);
      ctx.setLineDash([]);
      ctx.fillStyle = 'rgba(0, 209, 255, 0.1)';
      ctx.font = '11px system-ui';
      ctx.textAlign = 'left';
      ctx.fillText('L2/L3 共享域 (部分 Near-Atomic)', 35, 25);
    } else if (topology === 'cross-cluster') {
      // Draw two clusters
      ctx.strokeStyle = 'rgba(175, 82, 222, 0.3)';
      ctx.setLineDash([5, 5]);
      ctx.strokeRect(30, 30, 150, 100);
      ctx.strokeRect(190, 30, 150, 100);
      ctx.setLineDash([]);
      ctx.fillStyle = 'rgba(175, 82, 222, 0.5)';
      ctx.font = '10px system-ui';
      ctx.fillText('Cluster 0', 35, 25);
      ctx.fillText('Cluster 1', 195, 25);
      ctx.fillStyle = 'rgba(255, 149, 0, 0.5)';
      ctx.font = '9px system-ui';
      ctx.fillText('CHI 互连 (Far-Atomic)', 120, 145);
    } else { // cross-numa
      // Draw two NUMA nodes
      ctx.strokeStyle = 'rgba(255, 149, 0, 0.3)';
      ctx.setLineDash([5, 5]);
      ctx.strokeRect(20, 30, 120, 100);
      ctx.strokeRect(240, 30, 120, 100);
      ctx.setLineDash([]);
      ctx.fillStyle = 'rgba(255, 149, 0, 0.5)';
      ctx.font = '10px system-ui';
      ctx.fillText('NUMA Node 0', 25, 25);
      ctx.fillText('NUMA Node 1', 245, 25);
      ctx.fillStyle = 'rgba(255, 149, 0, 0.5)';
      ctx.font = '9px system-ui';
      ctx.fillText('CHI 互连 + Home Node', 130, 145);
    }

    ctx.restore();
  },

  /**
   * Render concurrency analysis section
   */
  renderConcurrencyAnalysis: function(scenario) {
    const ca = scenario.concurrencyAnalysis;
    if (!ca) return '';

    return `
      <div class="concurrency-analysis-section">
        <h4>并发核数分析</h4>

        <!-- Deployment Scale Cards -->
        <div class="deployment-scales-grid">
          ${this.renderScaleCard(ca.typicalDeployments.small, 'small')}
          ${this.renderScaleCard(ca.typicalDeployments.medium, 'medium')}
          ${this.renderScaleCard(ca.typicalDeployments.large, 'large')}
        </div>

        <!-- Performance by Cores Table -->
        <div class="performance-by-cores-section">
          <h5>不同核数下的性能表现</h5>
          <table class="cores-perf-table">
            <thead>
              <tr>
                <th>核心数</th>
                <th>Barrier 延迟</th>
                <th>吞吐量</th>
                <th>主要瓶颈</th>
              </tr>
            </thead>
            <tbody>
              ${ca.performanceByCores.map(p => `
                <tr>
                  <td><strong>${p.cores}</strong></td>
                  <td>${p.latency}</td>
                  <td>${p.throughput}</td>
                  <td>${p.bottleneck}</td>
                </tr>
              `).join('')}
            </tbody>
          </table>
        </div>
      </div>
    `;
  },

  /**
   * Render a single scale card
   */
  renderScaleCard: function(scale, type) {
    const icons = { small: '🖥️', medium: '🏢', large: '🌐' };
    const colors = {
      small: 'linear-gradient(135deg, rgba(52, 199, 89, 0.2), rgba(52, 199, 89, 0.05))',
      medium: 'linear-gradient(135deg, rgba(255, 149, 0, 0.2), rgba(255, 149, 0, 0.05))',
      large: 'linear-gradient(135deg, rgba(255, 59, 48, 0.2), rgba(255, 59, 48, 0.05))'
    };

    const titleMap = { small: '小规模', medium: '中等规模', large: '大规模' };

    return `
      <div class="scale-card ${type}" style="background: ${colors[type]}">
        <div class="scale-header">
          <span class="scale-icon">${icons[type]}</span>
          <h5>${titleMap[type]}</h5>
        </div>
        <div class="scale-info">
          <div class="core-count">${scale.cores}</div>
          <p class="scale-desc">${scale.description}</p>
          <div class="barrier-behavior">
            <span class="behavior-tag">${scale.barrierBehavior.split('，')[0]}</span>
            <p>${scale.barrierBehavior}</p>
          </div>
        </div>
      </div>
    `;
  },

  /**
   * Escape HTML
   */
  escapeHtml: function(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }
};

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', function() {
  BarrierPage.init();
});

// Export for global access
if (typeof window !== 'undefined') {
  window.BarrierPage = BarrierPage;
}
