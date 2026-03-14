// GTC 2026 Deep Leaks Visualization App
// 深度泄露信息可视化应用

document.addEventListener('DOMContentLoaded', function() {
  initWordCloud();
  initTimeline();
  initProducts();
  initDiscussionHeat();
  initDeepAnalysisSections();
  initComparisonSection();
  initMysterySection();
  initSupplySection();
  initFeynmanSection();
  initAIInferenceSection();
  initNVIDIAGroqDeal();
  initGroqComparison();
  initCloudPricing();
  initChanRumors();
  initOtherRumors();
  initValidationSection();
  initLeakersSection();
  initCredibilityGuide();
  initSourceTypes();
  initFilters();
  // 新增：汽车AI芯片相关
  initAutomotiveSections();
  // 新增：Omniverse相关
  initOmniverseSections();
});

// AI Inference Section
function initAIInferenceSection() {
  const grid = document.getElementById('inference-grid');
  if (!grid || typeof inferenceComparison === 'undefined') return;

  // 添加Groq LPU卡片
  if (leaksData.groqLPU) {
    const card = createInferenceCard(leaksData.groqLPU);
    grid.appendChild(card);
  }

  // 添加NVIDIA-Groq交易卡片
  if (leaksData.nvidiaGroqDeal) {
    const card = createInferenceCard(leaksData.nvidiaGroqDeal);
    grid.appendChild(card);
  }
}

function createInferenceCard(product) {
  const card = document.createElement('div');
  card.className = 'inference-card';

  const avgCred = Math.round(
    product.specs.reduce((sum, spec) => sum + spec.credibility, 0) / product.specs.length
  );

  const topSpecs = product.specs.slice(0, 3);
  const specsHTML = topSpecs.map(spec => `
    <div class="inference-stat">
      <div class="inference-stat-value" style="color: ${getCredibilityColor(spec.credibility)}">${spec.credibility}%</div>
      <div class="inference-stat-label">${spec.attr}</div>
    </div>
  `).join('');

  card.innerHTML = `
    <div class="inference-card-header">
      <div>
        <span class="inference-card-icon">${product.icon}</span>
        <span class="inference-card-title">${product.name}</span>
      </div>
      <span class="inference-card-value">${avgCred}%</span>
    </div>
    <div class="inference-card-desc">${product.description}</div>
    <div class="inference-stats">
      ${specsHTML}
    </div>
  `;

  return card;
}

// NVIDIA-Groq Deal Section
function initNVIDIAGroqDeal() {
  const container = document.getElementById('deal-content');
  if (!container || !leaksData.nvidiaGroqDeal) return;

  leaksData.nvidiaGroqDeal.specs.forEach(spec => {
    const item = document.createElement('div');
    item.className = 'deal-item';

    const icon = spec.credibility >= 90 ? '✅' : spec.credibility >= 70 ? '📌' : '💡';
    const credColor = getCredibilityColor(spec.credibility);

    item.innerHTML = `
      <div class="deal-item-icon">${icon}</div>
      <div class="deal-item-content">
        <div class="deal-item-title">${spec.attr}</div>
        <div class="deal-item-desc">${spec.info}</div>
      </div>
      <div class="deal-item-cred">
        <div class="deal-item-cred-value" style="color: ${credColor}">${spec.credibility}%</div>
        <div class="deal-item-cred-label">可信度</div>
      </div>
    `;

    container.appendChild(item);
  });
}

// Groq Comparison Section
function initGroqComparison() {
  const container = document.getElementById('groq-comparison');
  if (!container || typeof inferenceComparison === 'undefined') return;

  // Header
  const header = document.createElement('div');
  header.className = 'groq-comparison-row header';
  header.innerHTML = `
    <span>指标</span>
    <span class="groq-value groq">Groq LPU</span>
    <span class="groq-value nvidia">NVIDIA H100</span>
    <span>差距</span>
  `;
  container.appendChild(header);

  // Data rows
  inferenceComparison.comparison.forEach(item => {
    const row = document.createElement('div');
    row.className = 'groq-comparison-row';

    const isHighlight = item.ratio && item.ratio.includes('x');
    row.innerHTML = `
      <span>${item.metric}</span>
      <span class="groq-value groq">${item.groq}</span>
      <span class="groq-value nvidia">${item.nvidia}</span>
      <span class="groq-ratio ${isHighlight ? 'highlight' : ''}">${item.ratio}</span>
    `;
    container.appendChild(row);
  });
}

// Cloud Pricing Section
function initCloudPricing() {
  const grid = document.getElementById('cloud-grid');
  if (!grid || typeof cloudPricing === 'undefined') return;

  const providerIcons = {
    'AWS': '🟠',
    'Azure': '🔵',
    'Google Cloud': '🔷',
    'Oracle Cloud': '🔴'
  };

  cloudPricing.providers.forEach(provider => {
    const card = document.createElement('div');
    card.className = `cloud-card ${provider.name.toLowerCase().replace(' ', '')}`;

    const pricingHTML = Object.entries(provider.pricing).map(([gpu, price]) => `
      <div class="cloud-price-row">
        <span class="cloud-price-gpu">${gpu.toUpperCase()}</span>
        <span class="cloud-price-value">${price}/hr</span>
      </div>
    `).join('');

    const featuresHTML = provider.features.map(f => `
      <div class="cloud-feature">• ${f}</div>
    `).join('');

    card.innerHTML = `
      <div class="cloud-provider-name">
        ${providerIcons[provider.name] || '☁️'} ${provider.name}
      </div>
      <div class="cloud-relationship">${provider.relationship}</div>
      <div class="cloud-pricing-table">
        ${pricingHTML}
      </div>
      <div class="cloud-features">
        ${featuresHTML}
      </div>
    `;

    grid.appendChild(card);
  });
}

// Word Cloud Initialization
function initWordCloud() {
  const container = document.getElementById('wordcloud');
  if (!container) return;

  const canvas = document.createElement('canvas');
  canvas.width = 900;
  canvas.height = 400;
  container.appendChild(canvas);

  function getColor(weight) {
    if (weight >= 70) return '#00c853';
    if (weight >= 45) return '#76b900';
    return '#ffb800';
  }

  const options = {
    list: wordCloudData,
    gridSize: Math.round(16 * canvas.width / 1024),
    weightFactor: function(size) {
      return Math.pow(size, 1.1) * canvas.width / 400;
    },
    fontFamily: 'Segoe UI, Roboto, sans-serif',
    color: function(word, weight) {
      return getColor(weight);
    },
    rotateRatio: 0.3,
    rotationSteps: 2,
    backgroundColor: 'transparent',
    drawOutOfBound: false,
    shrinkToFit: true,
    hover: function(item, dimension, event) {
      canvas.style.cursor = 'pointer';
    },
    click: function(item) {
      showWordInfo(item[0], item[1]);
    }
  };

  WordCloud(canvas, options);
}

function showWordInfo(word, weight) {
  const relatedProduct = Object.values(leaksData).find(p =>
    p.name && (p.name.toLowerCase().includes(word.toLowerCase()) ||
    word.toLowerCase().includes(p.name.toLowerCase().split(' ')[0]))
  );

  let message = `"${word}" - 权重: ${weight}`;
  if (relatedProduct) {
    message += `\n相关产品: ${relatedProduct.name} (${relatedProduct.year})`;
  }
  alert(message);
}

// Timeline Initialization
function initTimeline() {
  const timeline = document.getElementById('timeline');
  if (!timeline || !timelineData) return;

  timeline.innerHTML = '';

  timelineData.forEach(item => {
    const timelineItem = document.createElement('div');
    timelineItem.className = 'timeline-item';

    let dotColor = item.color || 'var(--nvidia-green)';
    if (item.speculation) dotColor = '#ff6b6b';
    else if (item.rumor) dotColor = '#ffb800';

    timelineItem.innerHTML = `
      <div class="timeline-dot" style="background: ${dotColor}; box-shadow: 0 0 10px ${dotColor};"></div>
      <div class="timeline-year">${item.year}</div>
      <div class="timeline-event">${item.event}</div>
    `;

    timeline.appendChild(timelineItem);
  });
}

// Discussion Heat Section
function initDiscussionHeat() {
  const grid = document.getElementById('heat-grid');
  if (!grid) return;

  const heatData = [
    { name: 'RTX 5090', score: 95, trend: 'hot', mentions: 15678 },
    { name: '神秘芯片', score: 88, trend: 'hot', mentions: 9876 },
    { name: 'Rubin', score: 85, trend: 'stable', mentions: 12456 },
    { name: 'N1X笔记本', score: 78, trend: 'rising', mentions: 8234 },
    { name: 'Blackwell Ultra', score: 72, trend: 'stable', mentions: 6789 },
    { name: 'Feynman', score: 62, trend: 'rising', mentions: 4567 }
  ];

  heatData.forEach(item => {
    const card = document.createElement('div');
    card.className = 'heat-card';

    const heatClass = item.score >= 90 ? 'extreme' : item.score >= 75 ? 'hot' : item.score >= 50 ? 'rising' : 'stable';
    const trendIcon = item.trend === 'hot' ? '🔥' : item.trend === 'rising' ? '📈' : '➡️';

    card.innerHTML = `
      <div class="heat-header">
        <span class="heat-name">${item.name}</span>
        <span class="heat-score ${heatClass}">${item.score}</span>
      </div>
      <div class="heat-bar">
        <div class="heat-fill ${heatClass}" style="width: ${item.score}%"></div>
      </div>
      <div class="heat-meta">
        <span class="heat-trend">
          <span class="trend-icon">${trendIcon}</span>
          ${item.trend === 'hot' ? '极热' : item.trend === 'rising' ? '上升中' : '稳定'}
        </span>
        <span>${item.mentions.toLocaleString()} 讨论</span>
      </div>
    `;

    grid.appendChild(card);
  });
}

// Products Initialization
function initProducts() {
  const grid = document.getElementById('products-grid');
  if (!grid) return;

  Object.values(leaksData).forEach(product => {
    if (product.specs && product.name) {
      const card = createProductCard(product);
      grid.appendChild(card);
    }
  });
}

// Create Product Card
function createProductCard(product) {
  const card = document.createElement('div');
  card.className = 'product-card';
  card.dataset.id = product.id;

  // Calculate average credibility
  const avgCred = Math.round(
    product.specs.reduce((sum, spec) => sum + spec.credibility, 0) / product.specs.length
  );

  // Determine credibility category
  let credCategory = 'rumor';
  if (avgCred >= 90) credCategory = 'official';
  else if (avgCred >= 70) credCategory = 'high';

  card.dataset.category = credCategory;

  // Check if has Reddit or 4chan sources
  const hasReddit = product.sourceSummary && product.sourceSummary.some(s =>
    s.toLowerCase().includes('reddit') || s.toLowerCase().includes('r/')
  );
  const hasChan = product.sourceSummary && product.sourceSummary.some(s =>
    s.toLowerCase().includes('4chan') || s.toLowerCase().includes('chan')
  );

  if (hasReddit) card.dataset.sourceReddit = 'true';
  if (hasChan) card.dataset.sourceChan = 'true';

  // Build specs HTML
  const specsHTML = product.specs.map(spec => {
    const credColor = getCredibilityColor(spec.credibility);
    const validationBadge = spec.crossValidated ?
      `<span class="validation-badge level-${spec.validationLevel}">✓ ${getValidationLevelText(spec.validationLevel)}</span>` : '';
    const officialBadge = spec.official ? '<span class="official-badge">官方</span>' : '';
    const noteHTML = spec.note ? `<span class="spec-note">${spec.note}</span>` : '';
    const warningHTML = spec.warning ? `<span class="spec-warning">⚠️ ${spec.warning}</span>` : '';

    return `
      <div class="spec-row">
        <span class="spec-attr">${spec.attr}</span>
        <span class="spec-info">
          ${spec.info}
          ${officialBadge}
          ${validationBadge}
          ${noteHTML}
          ${warningHTML}
        </span>
        <div class="spec-cred">
          <div class="cred-bar">
            <div class="cred-fill" style="width: ${spec.credibility}%; background: ${credColor};"></div>
          </div>
          <span class="cred-value" style="color: ${credColor};">${spec.credibility}%</span>
        </div>
      </div>
    `;
  }).join('');

  // Build sources HTML
  const sourcesHTML = product.sourceSummary ?
    product.sourceSummary.map(source => {
      let sourceClass = '';
      if (source.toLowerCase().includes('nvidia') || source.toLowerCase().includes('官方')) sourceClass = 'official';
      else if (source.toLowerCase().includes('reddit') || source.startsWith('r/')) sourceClass = 'reddit';
      else if (source.toLowerCase().includes('twitter')) sourceClass = 'twitter';
      else if (source.toLowerCase().includes('4chan') || source.toLowerCase().includes('chan')) sourceClass = 'chan';
      else if (source.toLowerCase().includes('chiphell') || source.toLowerCase().includes('semiwiki') || source.toLowerCase().includes('overclock')) sourceClass = 'forum';
      else sourceClass = 'media';

      return `<span class="source-tag ${sourceClass}">${source}</span>`;
    }).join('') : '';

  // Heat indicator
  const heatClass = product.heatScore >= 90 ? 'extreme' : product.heatScore >= 75 ? 'hot' : product.heatScore >= 50 ? 'rising' : 'stable';

  card.innerHTML = `
    <div class="card-header">
      <span class="card-icon">${product.icon || '📦'}</span>
      <div class="card-title">
        <h3>${product.name}</h3>
        <span class="card-year">${product.year}</span>
      </div>
    </div>
    <p class="card-desc">${product.description}</p>
    <div class="card-heat">
      <span class="card-heat-score" style="color: ${getHeatColor(product.heatScore)}">${product.heatScore}</span>
      <span class="card-heat-label">讨论热度</span>
      <span class="card-mentions">${product.totalMentions ? product.totalMentions.toLocaleString() : 'N/A'} 提及</span>
    </div>
    <div class="specs-table">
      ${specsHTML}
    </div>
    <div class="card-footer">
      <div class="sources-label">信息来源:</div>
      <div class="source-tags">
        ${sourcesHTML}
      </div>
    </div>
  `;

  return card;
}

function getHeatColor(score) {
  if (score >= 90) return '#ff5252';
  if (score >= 75) return '#ff9800';
  if (score >= 50) return '#ffc107';
  return '#76b900';
}

function getCredibilityColor(value) {
  if (value >= 90) return '#00c853';
  if (value >= 70) return '#76b900';
  if (value >= 50) return '#ffb800';
  return '#ff6b6b';
}

function getValidationLevelText(level) {
  const texts = {
    1: '单一来源',
    2: '双重验证',
    3: '多重验证',
    4: '高度确认',
    5: '官方确认'
  };
  return texts[level] || '未知';
}

// Deep Analysis Sections
function initDeepAnalysisSections() {
  // RTX 5090 Analysis
  initAnalysisSection('rtx5090-analysis', leaksData.rtx5090);

  // N1X Analysis
  initAnalysisSection('n1x-analysis', leaksData.n1x);
}

function initAnalysisSection(containerId, product) {
  const container = document.getElementById(containerId);
  if (!container || !product) return;

  product.specs.forEach(spec => {
    const credClass = spec.credibility >= 80 ? 'high-cred' : spec.credibility >= 60 ? 'medium-cred' : 'low-cred';
    const validationBadge = spec.crossValidated ?
      `<span class="validation-badge level-${spec.validationLevel}">✓ ${getValidationLevelText(spec.validationLevel)}</span>` : '';
    const officialBadge = spec.official ? '<span class="official-badge">官方</span>' : '';
    const noteHTML = spec.note ? `<span class="spec-note">${spec.note}</span>` : '';
    const warningHTML = spec.warning ? `<span class="spec-warning">⚠️ ${spec.warning}</span>` : '';

    const specEl = document.createElement('div');
    specEl.className = `analysis-spec ${credClass}`;

    specEl.innerHTML = `
      <div class="analysis-spec-header">
        <span class="analysis-spec-attr">${spec.attr}</span>
        <div class="analysis-spec-cred">
          <span style="color: ${getCredibilityColor(spec.credibility)}">${spec.credibility}%</span>
          ${officialBadge}
          ${validationBadge}
        </div>
      </div>
      <div class="analysis-spec-info">${spec.info}</div>
      ${noteHTML}
      ${warningHTML}
    `;

    container.appendChild(specEl);
  });
}

// Comparison Section
function initComparisonSection() {
  const container = document.getElementById('rubin-comparison');
  if (!container || !leaksData.rubin) return;

  // Header
  const header = document.createElement('div');
  header.className = 'comparison-row header';
  header.innerHTML = `
    <span class="comparison-metric">指标</span>
    <span class="comparison-value blackwell">Blackwell</span>
    <span class="comparison-value rubin">Rubin</span>
    <span class="comparison-improvement">提升</span>
  `;
  container.appendChild(header);

  // Data rows
  const comparisonSpecs = leaksData.rubin.specs.filter(s => s.comparison);
  comparisonSpecs.forEach(spec => {
    const row = document.createElement('div');
    row.className = 'comparison-row';

    // Parse the info to extract Blackwell and Rubin values
    const info = spec.info;
    let blackwellVal = '-';
    let rubinVal = '-';
    let improvement = '-';

    if (info.includes('→')) {
      const parts = info.split('→');
      blackwellVal = parts[0].replace('Blackwell:', '').trim();
      rubinVal = parts[1].split('(')[0].replace('Rubin:', '').trim();
      const match = info.match(/\(([^)]+)\)/);
      if (match) improvement = match[1];
    } else {
      rubinVal = info;
    }

    const improvementClass = improvement.includes('提升') || improvement.includes('%') ? 'positive' :
                             improvement.includes('降低') ? 'positive' : 'neutral';

    row.innerHTML = `
      <span class="comparison-metric">${spec.attr}</span>
      <span class="comparison-value blackwell">${blackwellVal}</span>
      <span class="comparison-value rubin">${rubinVal}</span>
      <span class="comparison-improvement ${improvementClass}">${improvement}</span>
    `;

    container.appendChild(row);
  });
}

// Mystery Section
function initMysterySection() {
  const container = document.getElementById('mystery-content');
  if (!container || !leaksData.mysteryChip) return;

  leaksData.mysteryChip.specs.forEach(spec => {
    const card = document.createElement('div');
    const isOfficial = spec.official;
    const isSpeculation = spec.speculation;

    card.className = `mystery-card ${isOfficial ? 'official' : isSpeculation ? 'speculation' : ''}`;

    const credColor = getCredibilityColor(spec.credibility);
    const warningHTML = spec.warning ? `<div class="spec-warning" style="margin-top: 8px">⚠️ ${spec.warning}</div>` : '';

    card.innerHTML = `
      <div class="mystery-title">
        <span>${spec.attr}</span>
        <span style="color: ${credColor}; font-size: 14px">${spec.credibility}%</span>
      </div>
      <div class="mystery-desc">${spec.info}</div>
      ${warningHTML}
      <div class="mystery-meta">
        <span>${isOfficial ? '✓ 官方' : isSpeculation ? '🔮 推测' : '📰 媒体'}</span>
        <span>${spec.sources && spec.sources.length > 0 ? spec.sources.length + ' 来源' : '无来源'}</span>
      </div>
    `;

    container.appendChild(card);
  });
}

// Supply Section
function initSupplySection() {
  const container = document.getElementById('supply-content');
  if (!container || !leaksData.hbm4Supply) return;

  leaksData.hbm4Supply.specs.forEach(spec => {
    const card = document.createElement('div');
    card.className = 'supply-card';

    const credColor = getCredibilityColor(spec.credibility);
    const noteHTML = spec.note ? `<div class="spec-note" style="margin-top: 8px">${spec.note}</div>` : '';
    const warningHTML = spec.warning ? `<div class="spec-warning" style="margin-top: 8px">⚠️ ${spec.warning}</div>` : '';

    card.innerHTML = `
      <div class="supply-title">
        <span>${spec.attr}</span>
        <span style="color: ${credColor}; font-size: 14px">${spec.credibility}%</span>
      </div>
      <div class="supply-desc">${spec.info}</div>
      ${noteHTML}
      ${warningHTML}
    `;

    container.appendChild(card);
  });
}

// Feynman Section
function initFeynmanSection() {
  const container = document.getElementById('feynman-content');
  if (!container || !leaksData.feynman) return;

  leaksData.feynman.specs.forEach(spec => {
    const card = document.createElement('div');
    card.className = 'supply-card';

    const credColor = getCredibilityColor(spec.credibility);
    const noteHTML = spec.note ? `<div class="spec-note" style="margin-top: 8px">${spec.note}</div>` : '';
    const warningHTML = spec.warning ? `<div class="spec-warning" style="margin-top: 8px">⚠️ ${spec.warning}</div>` : '';

    card.innerHTML = `
      <div class="supply-title">
        <span>${spec.attr}</span>
        <span style="color: ${credColor}; font-size: 14px">${spec.credibility}%</span>
      </div>
      <div class="supply-desc">${spec.info}</div>
      ${noteHTML}
      ${warningHTML}
    `;

    container.appendChild(card);
  });
}

// 4chan Rumors
function initChanRumors() {
  const grid = document.getElementById('chan-grid');
  if (!grid || typeof chanRumors === 'undefined') return;

  chanRumors.forEach(rumor => {
    const card = document.createElement('div');
    card.className = 'chan-card';

    const credColor = getCredibilityColor(rumor.credibility);
    const noteHTML = rumor.note ? `<div class="spec-note" style="margin-top: 8px">${rumor.note}</div>` : '';

    card.innerHTML = `
      <div class="chan-header">
        <span class="chan-cred" style="color: ${credColor}">${rumor.credibility}%</span>
        <span class="badge badge-chan">${rumor.source}</span>
      </div>
      <div class="chan-info">${rumor.info}</div>
      <div class="chan-warning">${rumor.warning}</div>
      ${noteHTML}
      <div class="chan-meta">
        <span>📅 ${rumor.date}</span>
      </div>
    `;

    grid.appendChild(card);
  });
}

// Other Rumors
function initOtherRumors() {
  const grid = document.getElementById('rumors-grid');
  if (!grid || typeof otherRumors === 'undefined') return;

  otherRumors.forEach(rumor => {
    const card = document.createElement('div');
    card.className = 'rumor-card';

    const credColor = getCredibilityColor(rumor.credibility);
    const bgColor = credColor + '20';
    const noteHTML = rumor.note ? `<div class="rumor-note">${rumor.note}</div>` : '';
    const warningHTML = rumor.warning ? `<div class="spec-warning" style="margin-top: 8px">⚠️ ${rumor.warning}</div>` : '';

    card.innerHTML = `
      <div class="rumor-cred" style="background: ${bgColor}; color: ${credColor}; border: 2px solid ${credColor};">
        ${rumor.credibility}%
      </div>
      <div class="rumor-info">
        <div class="rumor-text">${rumor.info}</div>
        ${noteHTML}
        ${warningHTML}
      </div>
    `;

    grid.appendChild(card);
  });
}

// Validation Section
function initValidationSection() {
  const grid = document.getElementById('validation-grid');
  if (!grid || typeof crossValidations === 'undefined') return;

  Object.values(crossValidations).forEach(validation => {
    const item = document.createElement('div');
    item.className = 'validation-item';

    const levelColor = validation.validationLevel >= 4 ? '#00c853' :
                       validation.validationLevel >= 3 ? '#76b900' :
                       validation.validationLevel >= 2 ? '#ffb800' : '#ff6b6b';

    const sourcesHTML = validation.sources.map(sourceId => {
      const source = sourceLinks[sourceId];
      if (!source) return '';
      const typeInfo = sourceTypes[source.type] || sourceTypes.media;
      return `<span class="badge badge-${source.type}" style="font-size: 11px">${typeInfo.icon} ${source.title ? source.title.substring(0, 20) + '...' : sourceId}</span>`;
    }).join('');

    item.innerHTML = `
      <div class="validation-level">
        <div class="validation-level-num" style="color: ${levelColor}">${validation.validationLevel}</div>
        <div class="validation-level-text">${getValidationLevelText(validation.validationLevel)}</div>
      </div>
      <div class="validation-claim">
        <div class="validation-claim-text">${validation.claim}</div>
        <div class="validation-sources">${sourcesHTML}</div>
      </div>
    `;

    grid.appendChild(item);
  });
}

// Leakers Section
function initLeakersSection() {
  const grid = document.getElementById('leakers-grid');
  if (!grid || typeof knownLeakers === 'undefined') return;

  Object.values(knownLeakers).forEach(leaker => {
    const card = document.createElement('div');
    card.className = 'leaker-card';

    const accuracyColor = leaker.accuracy >= 80 ? '#00c853' :
                          leaker.accuracy >= 70 ? '#76b900' : '#ffb800';

    card.innerHTML = `
      <div class="leaker-avatar">${leaker.avatar}</div>
      <div class="leaker-name">${leaker.name}</div>
      <div class="leaker-platform">${leaker.platform}</div>
      <div class="leaker-stats">
        <div class="leaker-stat">
          <div class="leaker-stat-value" style="color: ${accuracyColor}">${leaker.accuracy}%</div>
          <div class="leaker-stat-label">准确率</div>
        </div>
        <div class="leaker-stat">
          <div class="leaker-stat-value">${leaker.specialty}</div>
          <div class="leaker-stat-label">专长领域</div>
        </div>
      </div>
    `;

    grid.appendChild(card);
  });
}

// Credibility Guide
function initCredibilityGuide() {
  const guide = document.getElementById('credibility-guide');
  if (!guide || typeof credibilityMethodology === 'undefined') return;

  credibilityMethodology.forEach(item => {
    const credItem = document.createElement('div');
    credItem.className = 'cred-item';

    credItem.innerHTML = `
      <span class="cred-label" style="color: ${item.color}">${item.level}</span>
      <span class="cred-label">${item.range}</span>
      <span class="cred-desc">${item.desc}</span>
    `;

    guide.appendChild(credItem);
  });
}

// Source Types
function initSourceTypes() {
  const grid = document.getElementById('source-types-grid');
  if (!grid || typeof sourceTypes === 'undefined') return;

  Object.entries(sourceTypes).forEach(([key, type]) => {
    const card = document.createElement('div');
    card.className = 'source-type-card';
    card.style.borderColor = type.color;

    card.innerHTML = `
      <div class="source-type-icon" style="color: ${type.color}">${type.icon}</div>
      <div class="source-type-name">${type.name}</div>
      <div class="source-type-cred">基础可信度: ${type.credibilityBase}%</div>
    `;

    grid.appendChild(card);
  });
}

// Filter functionality
function initFilters() {
  const filterBtns = document.querySelectorAll('.filter-btn');
  const cards = document.querySelectorAll('.product-card');

  filterBtns.forEach(btn => {
    btn.addEventListener('click', function() {
      filterBtns.forEach(b => b.classList.remove('active'));
      this.classList.add('active');

      const filter = this.dataset.filter;

      cards.forEach(card => {
        let show = false;

        switch(filter) {
          case 'all':
            show = true;
            break;
          case 'official':
            show = card.dataset.category === 'official';
            break;
          case 'high':
            show = card.dataset.category === 'official' || card.dataset.category === 'high';
            break;
          case 'rumor':
            show = card.dataset.category === 'rumor';
            break;
          case 'source-reddit':
            show = card.dataset.sourceReddit === 'true';
            break;
          case 'source-chan':
            show = card.dataset.sourceChan === 'true';
            break;
        }

        card.style.display = show ? 'block' : 'none';
      });
    });
  });
}

// Smooth scroll for navigation
document.querySelectorAll('a[href^="#"]').forEach(anchor => {
  anchor.addEventListener('click', function(e) {
    e.preventDefault();
    const target = document.querySelector(this.getAttribute('href'));
    if (target) {
      target.scrollIntoView({
        behavior: 'smooth',
        block: 'start'
      });
    }
  });
});

// ============================================
// 汽车 AI 芯片相关初始化
// ============================================
function initAutomotiveSections() {
  // DRIVE Thor Analysis
  if (typeof automotiveChips !== 'undefined' && automotiveChips.driveThor) {
    initAutomotiveAnalysisSection('drive-thor-analysis', automotiveChips.driveThor);
  }

  // Hyperion 10 Analysis
  if (typeof automotiveChips !== 'undefined' && automotiveChips.driveHyperion10) {
    initAutomotiveAnalysisSection('hyperion-analysis', automotiveChips.driveHyperion10);
  }

  // Alpamayo AI Analysis
  if (typeof automotiveChips !== 'undefined' && automotiveChips.alpamayoAI) {
    initAutomotiveAnalysisSection('alpamayo-analysis', automotiveChips.alpamayoAI);
  }

  // WeRide GXR Analysis
  if (typeof automotiveChips !== 'undefined' && automotiveChips.werideGXR) {
    initAutomotiveAnalysisSection('weride-analysis', automotiveChips.werideGXR);
  }

  // Partners Grid
  initPartnersGrid();
}

function initAutomotiveAnalysisSection(containerId, product) {
  const container = document.getElementById(containerId);
  if (!container || !product) return;

  product.specs.forEach(spec => {
    const credClass = spec.credibility >= 80 ? 'high-cred' : spec.credibility >= 60 ? 'medium-cred' : 'low-cred';
    const validationBadge = spec.crossValidated ?
      `<span class="validation-badge level-${spec.validationLevel}">✓ ${getValidationLevelText(spec.validationLevel)}</span>` : '';
    const officialBadge = spec.official ? '<span class="official-badge">官方</span>' : '';
    const noteHTML = spec.note ? `<span class="spec-note">${spec.note}</span>` : '';

    const specEl = document.createElement('div');
    specEl.className = `analysis-spec ${credClass}`;

    specEl.innerHTML = `
      <div class="analysis-spec-header">
        <span class="analysis-spec-attr">${spec.attr}</span>
        <div class="analysis-spec-cred">
          <span style="color: ${getCredibilityColor(spec.credibility)}">${spec.credibility}%</span>
          ${officialBadge}
          ${validationBadge}
        </div>
      </div>
      <div class="analysis-spec-info">${spec.info}</div>
      ${noteHTML}
    `;

    container.appendChild(specEl);
  });
}

function initPartnersGrid() {
  const grid = document.getElementById('partners-grid');
  if (!grid || typeof automakerPartners === 'undefined') return;

  // 中国车企
  if (automakerPartners.china) {
    const chinaSection = document.createElement('div');
    chinaSection.className = 'partners-section-group';
    chinaSection.innerHTML = '<h4>🇨🇳 中国车企</h4>';

    automakerPartners.china.forEach(partner => {
      const card = document.createElement('div');
      card.className = 'partner-card';
      card.innerHTML = `
        <div class="partner-name">${partner.name}</div>
        <div class="partner-chip">${partner.chip}</div>
        <div class="partner-status ${partner.status === '已确认' ? 'confirmed' : ''}">${partner.status}</div>
      `;
      chinaSection.appendChild(card);
    });

    grid.appendChild(chinaSection);
  }

  // 全球车企
  if (automakerPartners.global) {
    const globalSection = document.createElement('div');
    globalSection.className = 'partners-section-group';
    globalSection.innerHTML = '<h4>🌍 全球车企</h4>';

    automakerPartners.global.forEach(partner => {
      const card = document.createElement('div');
      card.className = 'partner-card';
      card.innerHTML = `
        <div class="partner-name">${partner.name}</div>
        <div class="partner-chip">${partner.chip}</div>
        <div class="partner-status ${partner.status === '已确认' ? 'confirmed' : ''}">${partner.status}</div>
      `;
      globalSection.appendChild(card);
    });

    grid.appendChild(globalSection);
  }
}

// ============================================
// Omniverse 相关初始化
// ============================================
function initOmniverseSections() {
  // Omniverse 主信息
  const omniverseGrid = document.querySelector('.omniverse-content');
  if (omniverseGrid && typeof omniversePlatform !== 'undefined') {
    omniversePlatform.specs.forEach(spec => {
      const specEl = document.createElement('div');
      specEl.className = 'omniverse-spec';

      const credColor = getCredibilityColor(spec.credibility);
      const officialBadge = spec.official ? '<span class="official-badge">官方</span>' : '';

      specEl.innerHTML = `
        <div class="omniverse-spec-header">
          <span class="omniverse-spec-attr">${spec.attr}</span>
          <span style="color: ${credColor}">${spec.credibility}%</span>
          ${officialBadge}
        </div>
        <div class="omniverse-spec-info">${spec.info}</div>
      `;

      omniverseGrid.appendChild(specEl);
    });
  }
}

// ============================================
// Jetson Thor 边缘AI芯片
// ============================================
function initJetsonSection() {
  // 如果有Jetson相关数据，在此初始化
}
