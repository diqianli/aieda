/**
 * ARM Lock Mechanisms - Detail Page Logic
 */

const DetailPage = {
  /**
   * Initialize the detail page
   */
  init: function() {
    const lockId = this.getLockIdFromURL();
    if (!lockId) {
      this.showError('未指定锁类型');
      return;
    }

    const lock = LockData.locks.find(l => l.id === lockId);
    if (!lock) {
      this.showError('未找到指定的锁类型: ' + lockId);
      return;
    }

    this.renderLockDetail(lock);
    this.renderRadarChart(lock);
    this.renderRelatedLocks(lock);
  },

  /**
   * Get lock ID from URL parameters
   */
  getLockIdFromURL: function() {
    const params = new URLSearchParams(window.location.search);
    return params.get('id');
  },

  /**
   * Show error message
   */
  showError: function(message) {
    const container = document.querySelector('.container');
    if (container) {
      container.innerHTML = `
        <div class="error-message">
          <h2>错误</h2>
          <p>${message}</p>
          <a href="index.html" class="back-link">返回主页</a>
        </div>
      `;
    }
  },

  /**
   * Render lock detail content
   */
  renderLockDetail: function(lock) {
    const category = LockData.categories.find(c => c.id === lock.category);

    // Update page title
    document.title = `${lock.name} - ARM Lock Mechanisms`;

    // Render header
    const headerEl = document.getElementById('lock-header');
    if (headerEl) {
      headerEl.innerHTML = `
        <a href="index.html" class="back-button">
          <span class="back-icon">←</span>
          <span>返回列表</span>
        </a>
        <div class="lock-title-area">
          <h1>${lock.name}</h1>
          <span class="name-en">${lock.nameEn}</span>
        </div>
        <span class="category-tag ${lock.category}" style="background: ${category.color}20; color: ${category.color}">
          ${category.icon} ${category.name}
        </span>
      `;
    }

    // Render main content
    const contentEl = document.getElementById('lock-content');
    if (contentEl) {
      contentEl.innerHTML = `
        ${this.renderDetailedPrinciple(lock)}
        ${this.renderUseCases(lock)}
        ${this.renderProsAndCons(lock)}
        ${this.renderArmImplementation(lock)}
        ${this.renderPseudocode(lock)}
        ${this.renderPerformancePoints(lock)}
        ${this.renderWarnings(lock)}
        ${this.renderKeywords(lock)}
      `;
    }
  },

  /**
   * Render detailed principle section
   */
  renderDetailedPrinciple: function(lock) {
    const detailedPrinciple = lock.detailedPrinciple || this.getDefaultDetailedPrinciple(lock);

    return `
      <section class="detail-section">
        <h2 class="section-title">
          <span class="section-icon">📖</span>
          详细原理
        </h2>
        <div class="section-content">
          <div class="principle-text">${this.formatText(detailedPrinciple)}</div>
        </div>
      </section>
    `;
  },

  /**
   * Get default detailed principle based on lock type
   */
  getDefaultDetailedPrinciple: function(lock) {
    const defaults = {
      'simple-spinlock': `简单自旋锁是最基础的锁实现之一，其核心思想是"忙等待"（Busy Waiting）。

<b>底层机制：</b>
当一个线程尝试获取锁时，它会不断地检查锁的状态。如果锁被占用，线程会一直循环等待（自旋），直到锁变为可用状态。这种机制完全在用户态完成，不需要内核介入。

<b>ARM架构实现：</b>
在ARMv8架构上，自旋锁通常使用LDXR（Load-Exclusive）和STXR（Store-Exclusive）指令对来实现原子操作。LDXR会标记对内存位置的独占访问，STXR只有在独占标记仍然有效时才能成功写入。

ARMv8.1-A引入了LSE（Large System Extension）原子指令集，如CASAL（Compare-and-Swap Acquire-Release），可以更高效地实现原子操作，减少总线流量。`,

      'ttas': `TTAS（Test-Then-Test-And-Set）锁是对简单自旋锁的优化，采用两阶段检查机制。

<b>工作原理：</b>
第一阶段使用普通读取指令检查锁状态，这不会产生缓存一致性流量。只有当发现锁空闲时，才执行第二阶段的原子CAS操作来获取锁。

<b>为什么更高效：</b>
简单自旋锁每次迭代都执行原子操作，会导致大量的缓存一致性流量（缓存行在多个核心间传递）。TTAS通过先读后原子操作的方式，大幅减少了这种流量。

<b>局限性：</b>
在高竞争场景下，当多个核心同时发现锁空闲并尝试CAS操作时，仍会产生竞争问题。`,

      'ticket-spinlock': `Ticket自旋锁采用类似银行叫号系统的机制，确保FIFO（先进先出）公平性。

<b>核心机制：</b>
锁维护两个计数器：next_ticket（下一个可用票号）和now_serving（当前服务号）。每个请求锁的线程先原子地获取一个票号，然后等待自己的票号被叫到。

<b>公平性保证：</b>
由于票号是递增分配的，且获取锁的顺序严格按照票号顺序，因此不会出现饥饿现象。每个等待者最终都能获得锁。

<b>ARM低功耗优化：</b>
使用WFE（Wait For Event）指令进入低功耗状态，配合SEV（Send Event）指令唤醒等待者，可以显著降低等待时的功耗。`,

      'mcs-spinlock': `MCS（Mellor-Crummey and Scott）队列自旋锁是一种可扩展的公平锁实现。

<b>链表队列结构：</b>
每个等待者在自己的本地节点上自旋，而不是在全局变量上自旋。等待者通过链表连接，形成一个队列。这彻底消除了缓存行颠簸问题。

<b>为什么NUMA友好：</b>
在NUMA系统中，不同CPU节点的内存访问延迟不同。MCS锁让每个CPU在自己的本地变量上自旋，避免了跨节点的缓存一致性流量。

<b>Linux内核应用：</b>
Linux内核的ih_queued_spinlock就是基于MCS设计的，是多核系统中的关键同步原语。`,

      'mutex': `互斥锁（Mutex）是一种阻塞锁，当竞争发生时会让出CPU，由内核调度其他线程。

<b>阻塞机制：</b>
与自旋锁不同，mutex在获取失败时不会忙等待，而是将当前线程加入等待队列，然后触发上下文切换让出CPU。当锁释放时，内核会唤醒等待队列中的一个线程。

<b>内核调度器交互：</b>
mutex的实现需要与内核调度器紧密配合。在Linux中，这通过futex系统调用实现，内核负责管理等待队列和线程唤醒。

<b>优先级继承：</b>
实时系统中常用带优先级继承的mutex（PI mutex），防止优先级反转问题。当高优先级线程等待低优先级线程持有的锁时，低优先级线程会临时继承高优先级。`,

      'futex': `Futex（Fast Userspace Mutex）是Linux特有的高效同步原语，结合了自旋锁和阻塞锁的优点。

<b>混合设计：</b>
无竞争时完全在用户态完成，使用原子操作快速获取锁（约10-20 cycles）。只有在竞争发生时才调用futex(2)系统调用进入内核态。

<b>状态值含义：</b>
- 0: 锁空闲
- 1: 锁被持有，无等待者
- 2: 锁被持有，有等待者

<b>为什么是通用最佳选择：</b>
在低竞争场景下性能接近自旋锁，在高竞争场景下能避免CPU空转。glibc的pthread_mutex底层就是基于futex实现的。`
    };

    return defaults[lock.id] || lock.principle;
  },

  /**
   * Render use cases section
   */
  renderUseCases: function(lock) {
    const useCases = lock.useCases || this.getDefaultUseCases(lock);

    return `
      <section class="detail-section">
        <h2 class="section-title">
          <span class="section-icon">💡</span>
          应用场景
        </h2>
        <div class="section-content">
          <div class="use-cases-grid">
            ${useCases.map(uc => `
              <div class="use-case-card">
                <h3 class="use-case-title">${uc.scenario}</h3>
                <p class="use-case-desc">${uc.description}</p>
                ${uc.codeExample ? `<pre class="use-case-code">${this.escapeHtml(uc.codeExample)}</pre>` : ''}
              </div>
            `).join('')}
          </div>
        </div>
      </section>
    `;
  },

  /**
   * Get default use cases based on lock type
   */
  getDefaultUseCases: function(lock) {
    const defaults = {
      'simple-spinlock': [
        {
          scenario: '中断处理程序',
          description: '在中断上下文中保护共享数据。中断处理程序不能睡眠，因此必须使用自旋锁而非阻塞锁。',
          codeExample: '// Linux内核中断处理\nspin_lock(&dev->lock);\nprocess_interrupt(dev);\nspin_unlock(&dev->lock);'
        },
        {
          scenario: '内核态短临界区',
          description: '保护只需几十个CPU周期的临界区，如修改链表指针或更新计数器。',
          codeExample: '// 短临界区示例\nspin_lock(&list_lock);\nlist_add(&new_node, &list);\nspin_unlock(&list_lock);'
        },
        {
          scenario: 'NMI上下文',
          description: '不可屏蔽中断（NMI）上下文中，连自旋锁都需要特殊处理（raw_spinlock）。'
        }
      ],
      'ttas': [
        {
          scenario: '中等竞争的短临界区',
          description: '当竞争程度中等时，TTAS在减少总线流量和获取锁速度之间取得了良好平衡。'
        },
        {
          scenario: 'NUMA系统优化',
          description: '在NUMA架构中，减少跨节点的缓存一致性流量对性能至关重要。'
        }
      ],
      'ticket-spinlock': [
        {
          scenario: '实时系统',
          description: '在实时系统中，公平性至关重要。Ticket锁确保所有线程按顺序获取锁，避免饥饿。',
          codeExample: '// RTOS中的使用\nticket_lock(&scheduler_lock);\nschedule_next();\nticket_unlock(&scheduler_lock);'
        },
        {
          scenario: '避免线程饥饿',
          description: '当系统需要保证所有线程都能公平地获取资源时，Ticket锁的FIFO特性非常关键。'
        }
      ],
      'mcs-spinlock': [
        {
          scenario: '高竞争多核系统',
          description: '当大量CPU核心竞争同一把锁时，MCS的本地自旋特性可以显著提升性能。'
        },
        {
          scenario: 'NUMA架构服务器',
          description: '在多插槽服务器中，MCS避免了跨插槽的缓存行颠簸，是Linux内核的首选。',
          codeExample: '// Linux内核 queued_spinlock\nqueued_spin_lock(&lock);\ncritical_section();\nqueued_spin_unlock(&lock);'
        }
      ],
      'mutex': [
        {
          scenario: '长临界区',
          description: '当临界区可能执行较长时间（如文件I/O、网络操作）时，使用mutex让其他线程有机会运行。'
        },
        {
          scenario: '可能阻塞的操作',
          description: '如果临界区内可能发生阻塞（如等待I/O、内存分配），必须使用mutex而非自旋锁。',
          codeExample: 'pthread_mutex_lock(&mutex);\n// 可能阻塞的操作\nwrite_to_file(data);\npthread_mutex_unlock(&mutex);'
        }
      ],
      'futex': [
        {
          scenario: '通用多线程程序',
          description: 'Futex是pthread_mutex的底层实现，适用于大多数用户态同步场景。',
          codeExample: '// glibc pthread_mutex使用futex\npthread_mutex_t mutex = PTHREAD_MUTEX_INITIALIZER;\npthread_mutex_lock(&mutex);\ncritical_section();\npthread_mutex_unlock(&mutex);'
        },
        {
          scenario: '竞争不确定的场景',
          description: '当无法预知锁竞争程度时，futex的自适应特性使其成为最安全的选择。'
        }
      ]
    };

    return defaults[lock.id] || [
      { scenario: '通用场景', description: lock.scenarios ? lock.scenarios.join(', ') : lock.principle }
    ];
  },

  /**
   * Render pros and cons section
   */
  renderProsAndCons: function(lock) {
    const prosAndCons = lock.prosAndCons || this.getDefaultProsAndCons(lock);

    return `
      <section class="detail-section">
        <h2 class="section-title">
          <span class="section-icon">⚖️</span>
          优缺点对比
        </h2>
        <div class="section-content">
          <div class="pros-cons-grid">
            <div class="pros-card">
              <h3 class="pros-title">✅ 优点</h3>
              <ul class="pros-list">
                ${prosAndCons.pros.map(pro => `<li>${pro}</li>`).join('')}
              </ul>
            </div>
            <div class="cons-card">
              <h3 class="cons-title">❌ 缺点</h3>
              <ul class="cons-list">
                ${prosAndCons.cons.map(con => `<li>${con}</li>`).join('')}
              </ul>
            </div>
          </div>
        </div>
      </section>
    `;
  },

  /**
   * Get default pros and cons based on lock type
   */
  getDefaultProsAndCons: function(lock) {
    const defaults = {
      'simple-spinlock': {
        pros: [
          '实现简单，代码量小',
          '无上下文切换开销',
          '延迟可预测（无竞争时）',
          '可在中断上下文使用',
          '无优先级反转问题'
        ],
        cons: [
          '浪费CPU周期（忙等待）',
          '高竞争下性能急剧下降',
          '功耗高（持续运行CPU）',
          '可能导致优先级反转',
          '缓存行颠簸问题严重'
        ]
      },
      'ttas': {
        pros: [
          '减少缓存一致性流量',
          '无竞争时性能优秀',
          '实现相对简单',
          '比简单自旋锁更高效'
        ],
        cons: [
          '高竞争下仍有问题',
          '不能保证公平性',
          '仍存在CPU空转'
        ]
      },
      'ticket-spinlock': {
        pros: [
          '保证FIFO公平性',
          '避免线程饥饿',
          '低功耗等待（WFE）',
          '延迟可预测'
        ],
        cons: [
          '所有等待者轮询同一变量',
          '缓存行颠簸仍存在',
          '票号可能溢出（需要特殊处理）'
        ]
      },
      'mcs-spinlock': {
        pros: [
          '本地自旋，无缓存颠簸',
          '极佳的NUMA可扩展性',
          '保证公平性（FIFO）',
          '高竞争下性能优异'
        ],
        cons: [
          '实现复杂',
          '每个CPU需要节点内存',
          '低竞争时有额外开销',
          '代码体积较大'
        ]
      },
      'mutex': {
        pros: [
          '不浪费CPU（阻塞等待）',
          '适合长临界区',
          '支持优先级继承',
          '低竞争时效率高'
        ],
        cons: [
          '上下文切换开销大（µs级）',
          '内核-用户态切换开销',
          '不适合短临界区',
          '不能在中断上下文使用'
        ]
      },
      'futex': {
        pros: [
          '无竞争时极快（~10-20 cycles）',
          '自适应竞争程度',
          '通用性最佳',
          'pthread_mutex底层实现'
        ],
        cons: [
          'Linux特有（非标准）',
          '竞争时系统调用开销',
          '实现细节复杂'
        ]
      }
    };

    return defaults[lock.id] || {
      pros: ['适用于特定场景'],
      cons: ['参见具体使用场景']
    };
  },

  /**
   * Render ARM implementation section
   */
  renderArmImplementation: function(lock) {
    return `
      <section class="detail-section">
        <h2 class="section-title">
          <span class="section-icon">🔧</span>
          ARM实现细节
        </h2>
        <div class="section-content">
          <div class="arm-impl-text">${lock.armImpl}</div>
          ${this.renderArmInstructions(lock)}
        </div>
      </section>
    `;
  },

  /**
   * Render ARM instruction examples
   */
  renderArmInstructions: function(lock) {
    const instructions = {
      'simple-spinlock': `
        <div class="instruction-example">
          <h4>ARMv8 LDXR/STXR 实现</h4>
          <pre class="asm-code">
spin_lock:
    ldxr    w1, [x0]        // 独占读取锁状态
    cbnz    w1, spin_lock   // 如果锁被占用，重试
    stxr    w2, wzr, [x0]   // 尝试原子写入0
    cbnz    w2, spin_lock   // 如果失败，重试
    dmb     ish             // 内存屏障
    ret</pre>
        </div>
        <div class="instruction-example">
          <h4>ARMv8.1 LSE 实现</h4>
          <pre class="asm-code">
spin_lock:
    mov     w1, #1
    swpala  w1, w1, [x0]    // 原子交换并获取
    cbnz    w1, spin_lock   // 如果旧值非0，重试
    ret</pre>
        </div>`,
      'ticket-spinlock': `
        <div class="instruction-example">
          <h4>WFE/SEV 低功耗等待</h4>
          <pre class="asm-code">
ticket_lock:
    ldaddal w1, w2, [x0]    // 原子获取票号
wait_loop:
    ldr     w3, [x0, #4]    // 读取当前服务号
    cmp     w2, w3          // 比较票号
    beq     acquired        // 相等则获得锁
    wfe                     // 低功耗等待
    b       wait_loop
acquired:
    ret

ticket_unlock:
    add     w1, w1, #1
    stlr    w1, [x0, #4]    // 更新服务号
    sev                     // 唤醒等待者
    ret</pre>
        </div>`
    };

    return instructions[lock.id] || '';
  },

  /**
   * Render pseudocode section
   */
  renderPseudocode: function(lock) {
    if (!lock.pseudocode) return '';

    return `
      <section class="detail-section">
        <h2 class="section-title">
          <span class="section-icon">📝</span>
          伪代码实现
        </h2>
        <div class="section-content">
          <div class="code-block">${this.highlightCode(lock.pseudocode)}</div>
        </div>
      </section>
    `;
  },

  /**
   * Render performance points section
   */
  renderPerformancePoints: function(lock) {
    return `
      <section class="detail-section">
        <h2 class="section-title">
          <span class="section-icon">⚡</span>
          性能敏感点
        </h2>
        <div class="section-content">
          <ul class="performance-points-detailed">
            ${lock.performancePoints.map(point => `
              <li>
                <span class="point-bullet">▸</span>
                <span class="point-text">${point}</span>
              </li>
            `).join('')}
          </ul>
        </div>
      </section>
    `;
  },

  /**
   * Render warnings section
   */
  renderWarnings: function(lock) {
    const warnings = lock.warnings || this.getDefaultWarnings(lock);

    return `
      <section class="detail-section warning-section">
        <h2 class="section-title">
          <span class="section-icon">⚠️</span>
          注意事项
        </h2>
        <div class="section-content">
          <ul class="warnings-list">
            ${warnings.map(warning => `
              <li>
                <span class="warning-icon">!</span>
                <span class="warning-text">${warning}</span>
              </li>
            `).join('')}
          </ul>
        </div>
      </section>
    `;
  },

  /**
   * Get default warnings based on lock type
   */
  getDefaultWarnings: function(lock) {
    const defaults = {
      'simple-spinlock': [
        '不要在持有自旋锁时调用可能睡眠的函数',
        '临界区应尽可能短（建议 < 100 cycles）',
        '避免在自旋锁临界区内调用其他可能获取锁的函数（死锁风险）',
        '在高竞争场景下考虑使用MCS锁替代'
      ],
      'ttas': [
        '仍然不能保证公平性',
        '高竞争时性能会下降',
        '不适合极长临界区'
      ],
      'ticket-spinlock': [
        '票号变量可能溢出，需要使用足够大的整数类型',
        '所有等待者仍轮询同一变量，存在缓存行颠簸',
        '不适合极高竞争场景（考虑MCS）'
      ],
      'mcs-spinlock': [
        '必须确保节点内存在锁持有期间保持有效',
        '每CPU需要预分配节点',
        '低竞争时额外开销可能不值得'
      ],
      'mutex': [
        '不能在中断上下文使用',
        '临界区过长会影响系统响应性',
        '注意优先级反转问题（使用PI mutex）'
      ],
      'futex': [
        '是Linux特有接口，移植性考虑',
        '状态值管理复杂，容易出错',
        '直接使用futex(2)需要小心处理边界情况'
      ]
    };

    return defaults[lock.id] || ['请参考具体锁类型的使用指南'];
  },

  /**
   * Render keywords section
   */
  renderKeywords: function(lock) {
    return `
      <section class="detail-section">
        <h2 class="section-title">
          <span class="section-icon">🏷️</span>
          相关关键字
        </h2>
        <div class="section-content">
          <div class="keywords">
            ${lock.keywords.map(k => `<span class="keyword">${k}</span>`).join('')}
          </div>
        </div>
      </section>
    `;
  },

  /**
   * Render radar chart for single lock
   */
  renderRadarChart: function(lock) {
    const canvas = document.getElementById('detail-radar-chart');
    if (!canvas || !lock.metrics) return;

    this.drawRadarChart(canvas, lock);
  },

  /**
   * Draw radar chart
   */
  drawRadarChart: function(canvas, lock) {
    const ctx = canvas.getContext('2d');
    const width = canvas.width = canvas.offsetWidth * 2;
    const height = canvas.height = canvas.offsetHeight * 2;
    ctx.scale(2, 2);

    const centerX = width / 4;
    const centerY = height / 4;
    const radius = Math.min(centerX, centerY) - 40;

    const metrics = [
      { key: 'latency', label: '延迟', color: '#2a71ff' },
      { key: 'throughput', label: '吞吐量', color: '#ff008a' },
      { key: 'fairness', label: '公平性', color: '#00d1ff' },
      { key: 'power', label: '功耗', color: '#ff9500' },
      { key: 'scalability', label: '可扩展性', color: '#34c759' }
    ];

    const numAxes = metrics.length;
    const angleStep = (2 * Math.PI) / numAxes;

    // Draw background circles
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.1)';
    ctx.lineWidth = 1;
    for (let i = 1; i <= 5; i++) {
      ctx.beginPath();
      ctx.arc(centerX, centerY, (radius / 5) * i, 0, 2 * Math.PI);
      ctx.stroke();
    }

    // Draw axes and labels
    ctx.fillStyle = 'rgba(255, 255, 255, 0.7)';
    ctx.font = '12px system-ui';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';

    metrics.forEach((metric, i) => {
      const angle = i * angleStep - Math.PI / 2;
      const x = centerX + Math.cos(angle) * radius;
      const y = centerY + Math.sin(angle) * radius;

      // Draw axis line
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.2)';
      ctx.beginPath();
      ctx.moveTo(centerX, centerY);
      ctx.lineTo(x, y);
      ctx.stroke();

      // Draw label
      const labelX = centerX + Math.cos(angle) * (radius + 25);
      const labelY = centerY + Math.sin(angle) * (radius + 25);
      ctx.fillStyle = metric.color;
      ctx.fillText(metric.label, labelX, labelY);
    });

    // Draw data polygon
    ctx.beginPath();
    metrics.forEach((metric, i) => {
      const value = lock.metrics[metric.key] || 0;
      const r = (value / 5) * radius;
      const angle = i * angleStep - Math.PI / 2;
      const x = centerX + Math.cos(angle) * r;
      const y = centerY + Math.sin(angle) * r;

      if (i === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    });
    ctx.closePath();

    // Fill with gradient
    const gradient = ctx.createRadialGradient(centerX, centerY, 0, centerX, centerY, radius);
    gradient.addColorStop(0, 'rgba(42, 113, 255, 0.3)');
    gradient.addColorStop(1, 'rgba(0, 209, 255, 0.1)');
    ctx.fillStyle = gradient;
    ctx.fill();

    // Stroke
    ctx.strokeStyle = '#2a71ff';
    ctx.lineWidth = 2;
    ctx.stroke();

    // Draw data points
    metrics.forEach((metric, i) => {
      const value = lock.metrics[metric.key] || 0;
      const r = (value / 5) * radius;
      const angle = i * angleStep - Math.PI / 2;
      const x = centerX + Math.cos(angle) * r;
      const y = centerY + Math.sin(angle) * r;

      ctx.beginPath();
      ctx.arc(x, y, 5, 0, 2 * Math.PI);
      ctx.fillStyle = metric.color;
      ctx.fill();
      ctx.strokeStyle = 'white';
      ctx.lineWidth = 2;
      ctx.stroke();
    });
  },

  /**
   * Render related locks section
   */
  renderRelatedLocks: function(currentLock) {
    const container = document.getElementById('related-locks');
    if (!container) return;

    // Find related locks by category and keywords
    const relatedLocks = LockData.locks.filter(lock => {
      if (lock.id === currentLock.id) return false;

      // Same category
      if (lock.category === currentLock.category) return true;

      // Shared keywords
      const sharedKeywords = lock.keywords.filter(k =>
        currentLock.keywords.some(ck => ck.toLowerCase() === k.toLowerCase())
      );
      if (sharedKeywords.length > 0) return true;

      return false;
    }).slice(0, 4);

    if (relatedLocks.length === 0) {
      container.style.display = 'none';
      return;
    }

    container.innerHTML = `
      <h2 class="section-title">
        <span class="section-icon">🔗</span>
        相关锁类型
      </h2>
      <div class="related-locks-grid">
        ${relatedLocks.map(lock => {
          const category = LockData.categories.find(c => c.id === lock.category);
          return `
            <a href="lock-detail.html?id=${lock.id}" class="related-lock-card">
              <h3>${lock.name}</h3>
              <span class="name-en">${lock.nameEn}</span>
              <span class="category-tag ${lock.category}" style="background: ${category.color}20; color: ${category.color}">
                ${category.name}
              </span>
            </a>
          `;
        }).join('')}
      </div>
    `;
  },

  /**
   * Format text with basic markdown-like syntax
   */
  formatText: function(text) {
    return text
      .replace(/\n\n/g, '</p><p>')
      .replace(/\n/g, '<br>')
      .replace(/<b>/g, '<strong>')
      .replace(/<\/b>/g, '</strong>');
  },

  /**
   * Escape HTML
   */
  escapeHtml: function(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  },

  /**
   * Highlight code syntax
   */
  highlightCode: function(code) {
    return code
      .replace(/\b(function|struct|if|while|return|true|false|null)\b/g, '<span class="keyword">$1</span>')
      .replace(/(\/\/.*)/g, '<span class="comment">$1</span>')
      .replace(/\b([a-z_]+)\s*\(/gi, '<span class="function">$1</span>(');
  }
};

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', function() {
  DetailPage.init();
});

// Export for global access
if (typeof window !== 'undefined') {
  window.DetailPage = DetailPage;
}
