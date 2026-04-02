/**
 * ARM Lock Mechanisms Visualization - Main Application
 */

const App = {
  state: {
    currentCategory: 'all',
    searchQuery: '',
    expandedCards: new Set()
  },

  /**
   * Initialize the application
   */
  init: function() {
    this.renderCategoryFilters();
    this.renderLocks();
    this.setupEventListeners();
    Diagrams.init();
  },

  /**
   * Setup event listeners
   */
  setupEventListeners: function() {
    // Search input
    const searchInput = document.getElementById('search-input');
    if (searchInput) {
      searchInput.addEventListener('input', (e) => {
        this.state.searchQuery = e.target.value;
        this.renderLocks();
      });
    }
  },

  /**
   * Render category filter buttons
   */
  renderCategoryFilters: function() {
    const container = document.getElementById('category-filters');
    if (!container) return;

    let html = `
      <button class="category-btn active" data-category="all" onclick="App.filterByCategory('all')">
        <span class="icon">◈</span>
        <span>全部</span>
      </button>
    `;

    LockData.categories.forEach(cat => {
      html += `
        <button class="category-btn" data-category="${cat.id}" onclick="App.filterByCategory('${cat.id}')">
          <span class="icon">${cat.icon}</span>
          <span>${cat.name}</span>
        </button>
      `;
    });

    container.innerHTML = html;
  },

  /**
   * Filter by category
   */
  filterByCategory: function(categoryId) {
    this.state.currentCategory = categoryId;

    // Update active button
    document.querySelectorAll('.category-btn').forEach(btn => {
      if (btn.dataset.category === categoryId) {
        btn.classList.add('active');
      } else {
        btn.classList.remove('active');
      }
    });

    this.renderLocks();
  },

  /**
   * Get filtered locks
   */
  getFilteredLocks: function() {
    let locks = LockData.locks;

    // Filter by category
    if (this.state.currentCategory !== 'all') {
      locks = locks.filter(lock => lock.category === this.state.currentCategory);
    }

    // Filter by search query
    if (this.state.searchQuery) {
      const query = this.state.searchQuery.toLowerCase();
      locks = locks.filter(lock =>
        lock.name.toLowerCase().includes(query) ||
        lock.nameEn.toLowerCase().includes(query) ||
        lock.keywords.some(k => k.toLowerCase().includes(query)) ||
        lock.principle.toLowerCase().includes(query)
      );
    }

    return locks;
  },

  /**
   * Render lock cards
   */
  renderLocks: function() {
    const container = document.getElementById('locks-container');
    if (!container) return;

    const locks = this.getFilteredLocks();

    if (locks.length === 0) {
      container.innerHTML = `
        <div class="no-results">
          <h3>未找到匹配的锁类型</h3>
          <p>尝试调整搜索条件或筛选器</p>
        </div>
      `;
      return;
    }

    let html = '';
    locks.forEach(lock => {
      const category = LockData.categories.find(c => c.id === lock.category);
      const isExpanded = this.state.expandedCards.has(lock.id);

      // Check CMH hints
      const hasShuh = lock.cmhHints?.shuh?.enabled;
      const hasStcph = lock.cmhHints?.stcph?.enabled;
      const isExperimental = lock.cmhHints?.shuh?.experimental || lock.cmhHints?.stcph?.experimental;
      const cmhBadges = (hasShuh || hasStcph) ? `
        <div class="cmh-badges">
          ${hasShuh ? `<span class="cmh-badge shuh${isExperimental ? ' experimental' : ''}" title="SHUH - Shared Update Hint${isExperimental ? ' (实验性)' : ''}">SHUH</span>` : ''}
          ${hasStcph ? `<span class="cmh-badge stcph${isExperimental ? ' experimental' : ''}" title="STCPH - Store Concurrent Priority Hint${isExperimental ? ' (实验性)' : ''}">STCPH</span>` : ''}
        </div>
      ` : '';

      html += `
        <div class="lock-card ${isExpanded ? 'expanded' : ''}" data-lock-id="${lock.id}">
          <div class="lock-card-header" onclick="App.toggleCard('${lock.id}')">
            <div class="lock-card-title">
              <h3>${lock.name}</h3>
              <span class="name-en">${lock.nameEn}</span>
              ${cmhBadges}
            </div>
            <div style="display: flex; align-items: center; gap: 12px;">
              <span class="category-tag ${lock.category}" style="background: ${category.color}20; color: ${category.color}">
                ${category.name}
              </span>
              <span class="expand-icon">▼</span>
            </div>
          </div>
          <div class="lock-card-content">
            <div class="lock-card-body">
              ${this.renderCardBody(lock)}
            </div>
          </div>
        </div>
      `;
    });

    container.innerHTML = html;
  },

  /**
   * Render card body content
   */
  renderCardBody: function(lock) {
    let html = '';

    // Principle
    html += `
      <div class="section">
        <div class="section-title">原理</div>
        <div class="section-text">${lock.principle}</div>
      </div>
    `;

    // ARM Implementation
    html += `
      <div class="section">
        <div class="section-title">ARM实现</div>
        <div class="section-text">${lock.armImpl}</div>
      </div>
    `;

    // Pseudocode
    if (lock.pseudocode) {
      html += `
        <div class="section">
          <div class="section-title">伪代码</div>
          <div class="code-block">${this.highlightCode(lock.pseudocode)}</div>
        </div>
      `;
    }

    // Performance Points
    html += `
      <div class="section">
        <div class="section-title">性能敏感点</div>
        <ul class="performance-points">
          ${lock.performancePoints.map(point => `<li>${point}</li>`).join('')}
        </ul>
      </div>
    `;

    // Scenarios
    html += `
      <div class="section">
        <div class="section-title">适用场景</div>
        <div class="tags">
          ${lock.scenarios.map(s => `<span class="tag">${s}</span>`).join('')}
        </div>
      </div>
    `;

    // Recommendations
    html += `
      <div class="section">
        <div class="section-title">推荐用法</div>
        <div class="section-text">${lock.recommendations}</div>
      </div>
    `;

    // Metrics mini chart
    if (lock.metrics) {
      html += `
        <div class="section">
          <div class="section-title">性能指标</div>
          ${this.renderMetricsMini(lock.metrics)}
        </div>
      `;
    }

    // Keywords
    html += `
      <div class="section">
        <div class="section-title">相关关键字</div>
        <div class="keywords">
          ${lock.keywords.map(k => `<span class="keyword">${k}</span>`).join('')}
        </div>
      </div>
    `;

    // Hardware Tendency (if available)
    if (lock.hardwareTendency) {
      html += `
        <div class="section">
          <div class="section-title">硬件原子倾向</div>
          ${this.renderHardwareTendency(lock)}
        </div>
      `;
    }

    // View Details Button
    html += `
      <div class="section">
        <a href="lock-detail.html?id=${lock.id}" class="view-details-btn">
          <span>查看完整详情</span>
          <span class="btn-icon">→</span>
        </a>
        ${lock.id === 'pthread-barrier' ? `
          <a href="barrier.html" class="barrier-deep-link">
            <span class="link-icon">📊</span>
            屏障深度分析
          </a>
        ` : ''}
      </div>
    `;

    return html;
  },

  /**
   * Render mini metrics display
   */
  renderMetricsMini: function(metrics) {
    const metricInfo = [
      { key: 'latency', label: '延迟', color: '#2a71ff' },
      { key: 'throughput', label: '吞吐量', color: '#ff008a' },
      { key: 'fairness', label: '公平性', color: '#00d1ff' },
      { key: 'power', label: '功耗', color: '#ff9500' },
      { key: 'scalability', label: '可扩展性', color: '#34c759' }
    ];

    return metricInfo.map(info => {
      const value = metrics[info.key] || 0;
      const width = (value / 5) * 100;
      return `
        <div class="metric-row">
          <span style="width: 60px; font-size: 11px; color: var(--text-muted)">${info.label}</span>
          <div class="metric-bar">
            <div class="metric-fill" style="width: ${width}%; background: ${info.color}"></div>
          </div>
          <span style="font-size: 11px; color: var(--text-secondary)">${value}/5</span>
        </div>
      `;
    }).join('');
  },

  /**
   * Render hardware tendency for lock card
   */
  renderHardwareTendency: function(lock) {
    const ht = lock.hardwareTendency;
    const meta = LockData.hardwareTendencyMeta[ht.type];

    // Calculate marker position
    const markerPos = ht.tendency + '%';
    const markerColor = ht.tendency < 40 ? '#00d1ff' : ht.tendency < 60 ? '#af52de' : '#ff9500';

    return `
      <div class="tendency-section">
        <div class="tendency-header">
          <div class="tendency-title">原子操作模式</div>
          <span class="tendency-badge ${ht.type}">
            <span>${meta.icon}</span>
            ${meta.nameEn}
          </span>
        </div>
        <div class="tendency-spectrum">
          <div class="tendency-marker" style="left: ${markerPos}; border-color: ${markerColor}"></div>
        </div>
        <div class="tendency-labels">
          <div class="tendency-label"><span style="color: #00d1ff">◉</span> 近端</div>
          <div class="tendency-value">${ht.tendency}%</div>
          <div class="tendency-label"><span style="color: #ff9500">◎</span> 远端</div>
        </div>
        <div class="tendency-description">${ht.description}</div>
      </div>
    `;
  },

  /**
   * Highlight code syntax
   */
  highlightCode: function(code) {
    // Simple syntax highlighting
    return code
      .replace(/\b(function|struct|if|while|return|true|false|null)\b/g, '<span class="keyword">$1</span>')
      .replace(/(\/\/.*)/g, '<span class="comment">$1</span>')
      .replace(/\b([a-z_]+)\s*\(/gi, '<span class="function">$1</span>(');
  },

  /**
   * Toggle card expansion
   */
  toggleCard: function(lockId) {
    if (this.state.expandedCards.has(lockId)) {
      this.state.expandedCards.delete(lockId);
    } else {
      this.state.expandedCards.add(lockId);
    }

    const card = document.querySelector(`.lock-card[data-lock-id="${lockId}"]`);
    if (card) {
      card.classList.toggle('expanded');
    }
  },

  /**
   * Expand all cards
   */
  expandAll: function() {
    LockData.locks.forEach(lock => {
      this.state.expandedCards.add(lock.id);
    });
    document.querySelectorAll('.lock-card').forEach(card => {
      card.classList.add('expanded');
    });
  },

  /**
   * Collapse all cards
   */
  collapseAll: function() {
    this.state.expandedCards.clear();
    document.querySelectorAll('.lock-card').forEach(card => {
      card.classList.remove('expanded');
    });
  },

  /**
   * Scroll to section
   */
  scrollToSection: function(sectionId) {
    const section = document.getElementById(sectionId);
    if (section) {
      section.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  },

  /**
   * Export data as JSON
   */
  exportData: function() {
    const data = {
      categories: LockData.categories,
      locks: LockData.locks,
      exportedAt: new Date().toISOString()
    };

    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'arm-locks-data.json';
    a.click();
    URL.revokeObjectURL(url);
  },

  /**
   * Get lock by ID
   */
  getLockById: function(lockId) {
    return LockData.locks.find(lock => lock.id === lockId);
  },

  /**
   * Search locks by keyword
   */
  searchByKeyword: function(keyword) {
    this.state.searchQuery = keyword;
    const searchInput = document.getElementById('search-input');
    if (searchInput) {
      searchInput.value = keyword;
    }
    this.renderLocks();
  }
};

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', function() {
  App.init();
});

// Export for global access
if (typeof window !== 'undefined') {
  window.App = App;
}
