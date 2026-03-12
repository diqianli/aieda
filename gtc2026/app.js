// GTC 2026 Leaks Visualization App

document.addEventListener('DOMContentLoaded', function() {
  initWordCloud();
  initTimeline();
  initProducts();
  initRumors();
  initFilters();
});

// Word Cloud Initialization
function initWordCloud() {
  const container = document.getElementById('wordcloud');
  const canvas = document.createElement('canvas');
  canvas.width = 900;
  canvas.height = 400;
  container.appendChild(canvas);

  // Color function based on weight
  function getColor(weight) {
    if (weight >= 70) {
      return '#00c853'; // High - Green
    } else if (weight >= 45) {
      return '#76b900'; // Medium - NVIDIA Green
    } else {
      return '#ffb800'; // Low - Yellow
    }
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

// Show word info tooltip
function showWordInfo(word, weight) {
  // Find related product
  const relatedProduct = leaksData.products.find(p =>
    p.name.toLowerCase().includes(word.toLowerCase()) ||
    word.toLowerCase().includes(p.name.toLowerCase().split(' ')[0])
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

  timelineData.forEach(item => {
    const timelineItem = document.createElement('div');
    timelineItem.className = 'timeline-item';

    const dotColor = item.color || 'var(--nvidia-green)';

    timelineItem.innerHTML = `
      <div class="timeline-dot" style="background: ${dotColor}; box-shadow: 0 0 10px ${dotColor};"></div>
      <div class="timeline-year">${item.year}</div>
      <div class="timeline-event">${item.event}</div>
    `;

    timeline.appendChild(timelineItem);
  });
}

// Products Initialization
function initProducts() {
  const grid = document.getElementById('products-grid');

  leaksData.products.forEach(product => {
    const card = createProductCard(product);
    grid.appendChild(card);
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
  if (avgCred >= 95) credCategory = 'confirmed';
  else if (avgCred >= 70) credCategory = 'high';

  card.dataset.category = credCategory;

  // Build specs HTML
  const specsHTML = product.specs.map(spec => {
    const credColor = getCredibilityColor(spec.credibility);
    return `
      <div class="spec-row">
        <span class="spec-attr">${spec.attr}</span>
        <span class="spec-info">${spec.info}</span>
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
  const sourcesHTML = product.sources.map(source =>
    `<span class="source-tag">${source}</span>`
  ).join('');

  card.innerHTML = `
    <div class="card-header">
      <span class="card-icon">${product.icon}</span>
      <div class="card-title">
        <h3>${product.name}</h3>
        <span class="card-year">${product.year}</span>
      </div>
    </div>
    <p class="card-desc">${product.description}</p>
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

// Get credibility color
function getCredibilityColor(value) {
  if (value >= 90) return '#00c853';
  if (value >= 70) return '#76b900';
  if (value >= 50) return '#ffb800';
  return '#ff6b6b';
}

// Rumors Initialization
function initRumors() {
  const grid = document.getElementById('rumors-grid');

  leaksData.rumors.forEach(rumor => {
    const card = document.createElement('div');
    card.className = 'rumor-card';

    const credColor = getCredibilityColor(rumor.credibility);
    const bgColor = credColor + '20'; // Add transparency

    card.innerHTML = `
      <div class="rumor-cred" style="background: ${bgColor}; color: ${credColor}; border: 2px solid ${credColor};">
        ${rumor.credibility}%
      </div>
      <div class="rumor-info">
        <div class="rumor-text">${rumor.info}</div>
        <div class="rumor-note">${rumor.note}</div>
      </div>
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
      // Update active button
      filterBtns.forEach(b => b.classList.remove('active'));
      this.classList.add('active');

      const filter = this.dataset.filter;

      cards.forEach(card => {
        if (filter === 'all') {
          card.style.display = 'block';
        } else {
          const category = card.dataset.category;
          if (filter === 'confirmed' && category === 'confirmed') {
            card.style.display = 'block';
          } else if (filter === 'high' && (category === 'confirmed' || category === 'high')) {
            card.style.display = 'block';
          } else if (filter === 'rumor' && category === 'rumor') {
            card.style.display = 'block';
          } else if (filter === 'rumor') {
            // Show all non-confirmed as rumors
            card.style.display = 'block';
          } else {
            card.style.display = 'none';
          }
        }
      });

      // Handle filter logic properly
      cards.forEach(card => {
        const category = card.dataset.category;
        let show = false;

        switch(filter) {
          case 'all':
            show = true;
            break;
          case 'confirmed':
            show = category === 'confirmed';
            break;
          case 'high':
            show = category === 'confirmed' || category === 'high';
            break;
          case 'rumor':
            show = category === 'rumor';
            break;
        }

        card.style.display = show ? 'block' : 'none';
      });
    });
  });
}

// Animate credibility bars on scroll
function animateCredBars() {
  const bars = document.querySelectorAll('.cred-fill');

  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        entry.target.style.width = entry.target.style.width || '0%';
      }
    });
  }, { threshold: 0.5 });

  bars.forEach(bar => observer.observe(bar));
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
