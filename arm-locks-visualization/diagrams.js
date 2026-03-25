/**
 * Interactive Diagrams for ARM Lock Visualization
 * Includes radar charts and decision tree visualization
 */

const Diagrams = {
  // Color palette
  colors: {
    primary: '#2a71ff',
    secondary: '#ff008a',
    tertiary: '#00d1ff',
    quaternary: '#ff9500',
    quinary: '#34c759',
    grid: 'rgba(255, 255, 255, 0.1)',
    text: 'rgba(255, 255, 255, 0.7)',
    textMuted: 'rgba(255, 255, 255, 0.5)'
  },

  // Selected locks for comparison
  selectedLocks: [],

  /**
   * Draw radar chart
   */
  drawRadarChart: function(canvasId, locks = []) {
    const canvas = document.getElementById(canvasId);
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    const rect = canvas.getBoundingClientRect();

    // Set canvas size for high DPI
    const dpr = window.devicePixelRatio || 1;
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    ctx.scale(dpr, dpr);

    const width = rect.width;
    const height = rect.height;
    const centerX = width / 2;
    const centerY = height / 2;
    const radius = Math.min(width, height) / 2 - 50;

    // Metrics
    const metrics = ['延迟', '吞吐量', '公平性', '功耗', '可扩展性'];
    const metricKeys = ['latency', 'throughput', 'fairness', 'power', 'scalability'];
    const numMetrics = metrics.length;
    const angleStep = (2 * Math.PI) / numMetrics;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Draw background circles
    for (let i = 5; i >= 1; i--) {
      const r = (radius * i) / 5;
      ctx.beginPath();
      ctx.arc(centerX, centerY, r, 0, 2 * Math.PI);
      ctx.fillStyle = `rgba(255, 255, 255, ${0.02 * i})`;
      ctx.fill();
      ctx.strokeStyle = this.colors.grid;
      ctx.lineWidth = 1;
      ctx.stroke();
    }

    // Draw axes and labels
    for (let i = 0; i < numMetrics; i++) {
      const angle = i * angleStep - Math.PI / 2;
      const x = centerX + Math.cos(angle) * radius;
      const y = centerY + Math.sin(angle) * radius;

      // Axis line
      ctx.beginPath();
      ctx.moveTo(centerX, centerY);
      ctx.lineTo(x, y);
      ctx.strokeStyle = this.colors.grid;
      ctx.lineWidth = 1;
      ctx.stroke();

      // Label
      const labelX = centerX + Math.cos(angle) * (radius + 25);
      const labelY = centerY + Math.sin(angle) * (radius + 25);
      ctx.font = '12px system-ui';
      ctx.fillStyle = this.colors.text;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(metrics[i], labelX, labelY);
    }

    // Color palette for different locks
    const lockColors = [
      { fill: 'rgba(42, 113, 255, 0.2)', stroke: '#2a71ff' },
      { fill: 'rgba(255, 0, 138, 0.2)', stroke: '#ff008a' },
      { fill: 'rgba(0, 209, 255, 0.2)', stroke: '#00d1ff' },
      { fill: 'rgba(255, 149, 0, 0.2)', stroke: '#ff9500' },
      { fill: 'rgba(52, 199, 89, 0.2)', stroke: '#34c759' }
    ];

    // Draw data for each lock
    locks.forEach((lock, lockIndex) => {
      if (!lock.metrics) return;

      const color = lockColors[lockIndex % lockColors.length];
      const points = [];

      // Calculate points
      metricKeys.forEach((key, i) => {
        const value = lock.metrics[key] || 0;
        const angle = i * angleStep - Math.PI / 2;
        const r = (radius * value) / 5;
        points.push({
          x: centerX + Math.cos(angle) * r,
          y: centerY + Math.sin(angle) * r
        });
      });

      // Draw filled polygon
      ctx.beginPath();
      points.forEach((point, i) => {
        if (i === 0) {
          ctx.moveTo(point.x, point.y);
        } else {
          ctx.lineTo(point.x, point.y);
        }
      });
      ctx.closePath();
      ctx.fillStyle = color.fill;
      ctx.fill();
      ctx.strokeStyle = color.stroke;
      ctx.lineWidth = 2;
      ctx.stroke();

      // Draw points
      points.forEach(point => {
        ctx.beginPath();
        ctx.arc(point.x, point.y, 4, 0, 2 * Math.PI);
        ctx.fillStyle = color.stroke;
        ctx.fill();
      });
    });

    // Draw title
    ctx.font = '600 14px system-ui';
    ctx.fillStyle = this.colors.text;
    ctx.textAlign = 'center';
    ctx.fillText('性能对比雷达图', centerX, 20);
  },

  /**
   * Draw bar chart for single lock comparison
   */
  drawBarChart: function(canvasId, lock) {
    const canvas = document.getElementById(canvasId);
    if (!canvas || !lock || !lock.metrics) return;

    const ctx = canvas.getContext('2d');
    const rect = canvas.getBoundingClientRect();

    const dpr = window.devicePixelRatio || 1;
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    ctx.scale(dpr, dpr);

    const width = rect.width;
    const height = rect.height;
    const padding = { top: 30, right: 20, bottom: 40, left: 80 };
    const chartWidth = width - padding.left - padding.right;
    const chartHeight = height - padding.top - padding.bottom;

    const metrics = [
      { key: 'latency', label: '延迟', color: '#2a71ff' },
      { key: 'throughput', label: '吞吐量', color: '#ff008a' },
      { key: 'fairness', label: '公平性', color: '#00d1ff' },
      { key: 'power', label: '功耗', color: '#ff9500' },
      { key: 'scalability', label: '可扩展性', color: '#34c759' }
    ];

    const barHeight = chartHeight / metrics.length - 10;

    // Clear
    ctx.clearRect(0, 0, width, height);

    // Draw bars
    metrics.forEach((metric, i) => {
      const value = lock.metrics[metric.key] || 0;
      const barWidth = (chartWidth * value) / 5;
      const y = padding.top + i * (chartHeight / metrics.length) + 5;

      // Background bar
      ctx.fillStyle = 'rgba(255, 255, 255, 0.1)';
      ctx.fillRect(padding.left, y, chartWidth, barHeight);

      // Value bar with gradient
      const gradient = ctx.createLinearGradient(padding.left, 0, padding.left + barWidth, 0);
      gradient.addColorStop(0, metric.color);
      gradient.addColorStop(1, this.adjustColor(metric.color, 0.7));
      ctx.fillStyle = gradient;
      ctx.fillRect(padding.left, y, barWidth, barHeight);

      // Label
      ctx.font = '12px system-ui';
      ctx.fillStyle = this.colors.text;
      ctx.textAlign = 'right';
      ctx.textBaseline = 'middle';
      ctx.fillText(metric.label, padding.left - 10, y + barHeight / 2);

      // Value
      ctx.textAlign = 'left';
      ctx.fillText(value + '/5', padding.left + barWidth + 8, y + barHeight / 2);
    });
  },

  /**
   * Adjust color brightness
   */
  adjustColor: function(color, factor) {
    const hex = color.replace('#', '');
    const r = Math.round(parseInt(hex.substr(0, 2), 16) * factor);
    const g = Math.round(parseInt(hex.substr(2, 2), 16) * factor);
    const b = Math.round(parseInt(hex.substr(4, 2), 16) * factor);
    return `rgb(${r}, ${g}, ${b})`;
  },

  /**
   * Initialize interactive decision tree
   */
  initDecisionTree: function(containerId) {
    const container = document.getElementById(containerId);
    if (!container) return;

    this.decisionTreeState = {
      currentStep: 0,
      answers: {}
    };

    this.renderDecisionTree(container);
  },

  /**
   * Decision tree steps
   */
  decisionSteps: [
    {
      question: '临界区长度?',
      options: [
        { value: 'short', label: '短 (< 100 cycles)' },
        { value: 'medium', label: '中等 (100-1000 cycles)' },
        { value: 'long', label: '长 (> 1000 cycles)' }
      ]
    },
    {
      question: '竞争程度?',
      options: [
        { value: 'low', label: '低竞争' },
        { value: 'medium', label: '中等竞争' },
        { value: 'high', label: '高竞争' },
        { value: 'unknown', label: '不确定' }
      ],
      condition: (answers) => answers[0] !== 'short'
    },
    {
      question: '需要公平性保证?',
      options: [
        { value: 'yes', label: '是，需要FIFO' },
        { value: 'no', label: '否，性能优先' }
      ],
      condition: (answers) => answers[0] !== 'short' && answers[1] === 'high'
    },
    {
      question: '高竞争下的优先考虑?',
      options: [
        { value: 'scalability', label: '可扩展性 (NUMA)' },
        { value: 'simplicity', label: '简单实现' }
      ],
      condition: (answers) => answers[0] === 'short' && answers[1] === 'high'
    }
  ],

  /**
   * Render decision tree
   */
  renderDecisionTree: function(container) {
    const state = this.decisionTreeState;
    let html = '<div class="interactive-tree">';

    // Render completed steps
    for (let i = 0; i < state.currentStep; i++) {
      const step = this.decisionSteps[i];
      if (step.condition && !step.condition(state.answers)) continue;

      html += `
        <div class="tree-level">
          <div class="decision-question">${step.question}</div>
        </div>
        <div class="tree-level">
          <div class="tree-option selected">${step.options.find(o => o.value === state.answers[i])?.label}</div>
        </div>
      `;
    }

    // Render current step
    const currentStepData = this.decisionSteps[state.currentStep];

    if (currentStepData) {
      // Check condition
      if (currentStepData.condition && !currentStepData.condition(state.answers)) {
        // Skip this step
        state.currentStep++;
        this.renderDecisionTree(container);
        return;
      }

      html += `
        <div class="tree-level">
          <div class="decision-question">${currentStepData.question}</div>
        </div>
        <div class="tree-level">
          ${currentStepData.options.map(opt => `
            <div class="tree-option" onclick="Diagrams.selectOption(${state.currentStep}, '${opt.value}')">
              ${opt.label}
            </div>
          `).join('')}
        </div>
      `;
    } else {
      // Show result
      const result = this.getDecisionResult(state.answers);
      html += `
        <div class="tree-result">
          <h3>推荐: ${result.name}</h3>
          <p>${result.recommendation}</p>
          <button class="tree-option" onclick="Diagrams.resetDecisionTree()" style="margin-top: 12px;">
            重新开始
          </button>
        </div>
      `;
    }

    html += '</div>';
    container.innerHTML = html;
  },

  /**
   * Select option in decision tree
   */
  selectOption: function(step, value) {
    this.decisionTreeState.answers[step] = value;
    this.decisionTreeState.currentStep++;
    this.renderDecisionTree(document.getElementById('decision-tree'));
  },

  /**
   * Reset decision tree
   */
  resetDecisionTree: function() {
    this.decisionTreeState = {
      currentStep: 0,
      answers: {}
    };
    this.renderDecisionTree(document.getElementById('decision-tree'));
  },

  /**
   * Get decision result based on answers
   */
  getDecisionResult: function(answers) {
    const criticalSection = answers[0];
    const competition = answers[1];
    const fairness = answers[2];
    const priority = answers[3];

    // Decision logic
    if (criticalSection === 'short') {
      if (competition === 'high') {
        if (priority === 'scalability') {
          return {
            id: 'mcs-spinlock',
            name: 'MCS队列自旋锁',
            recommendation: '极佳的可扩展性和公平性，适合NUMA系统'
          };
        } else {
          return {
            id: 'ttas',
            name: 'TTAS锁',
            recommendation: '简单高效，减少缓存一致性流量'
          };
        }
      } else {
        return {
          id: 'simple-spinlock',
          name: '简单自旋锁',
          recommendation: '极短临界区的最佳选择'
        };
      }
    } else {
      // Medium or long critical section
      if (competition === 'unknown' || competition === 'low') {
        return {
          id: 'futex',
          name: 'Futex',
          recommendation: '通用最佳选择，无竞争时用户态快速路径'
        };
      } else if (competition === 'high') {
        if (fairness === 'yes') {
          return {
            id: 'ticket-spinlock',
            name: 'Ticket自旋锁',
            recommendation: '严格FIFO公平性保证'
          };
        } else {
          return {
            id: 'mutex',
            name: 'Mutex',
            recommendation: '标准阻塞锁，适合长临界区'
          };
        }
      } else {
        return {
          id: 'futex',
          name: 'Futex',
          recommendation: '通用场景的最佳选择'
        };
      }
    }
  },

  /**
   * Toggle lock selection for comparison
   */
  toggleLockSelection: function(lockId) {
    const index = this.selectedLocks.indexOf(lockId);
    if (index === -1) {
      if (this.selectedLocks.length < 5) {
        this.selectedLocks.push(lockId);
      }
    } else {
      this.selectedLocks.splice(index, 1);
    }
    this.updateComparisonChart();
    this.updateLockSelectorUI();
  },

  /**
   * Update comparison chart
   */
  updateComparisonChart: function() {
    const locks = this.selectedLocks.map(id =>
      LockData.locks.find(l => l.id === id)
    ).filter(Boolean);

    this.drawRadarChart('radar-chart', locks);
  },

  /**
   * Update lock selector UI
   */
  updateLockSelectorUI: function() {
    document.querySelectorAll('.lock-selector-btn').forEach(btn => {
      const lockId = btn.dataset.lockId;
      if (this.selectedLocks.includes(lockId)) {
        btn.classList.add('active');
      } else {
        btn.classList.remove('active');
      }
    });
  },

  /**
   * Initialize all diagrams
   */
  init: function() {
    // Initialize with default locks
    this.selectedLocks = ['simple-spinlock', 'futex', 'mcs-spinlock'];
    this.updateComparisonChart();
    this.initDecisionTree('decision-tree');

    // Handle resize
    window.addEventListener('resize', () => {
      this.updateComparisonChart();
    });
  }
};

// Export
if (typeof module !== 'undefined' && module.exports) {
  module.exports = Diagrams;
}
