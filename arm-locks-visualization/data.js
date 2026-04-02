/**
 * ARM Lock Mechanisms Data Definitions
 * Contains all lock types, categories, performance metrics, and pseudocode
 */

const LockData = {
  // Hardware tendency metadata
  hardwareTendencyMeta: {
    near: { name: '近端原子', nameEn: 'Near-Atomic', color: '#00d1ff', icon: '◉',
             desc: '在本核L1缓存命中时执行，原子操作在本地核心的L1内存子系统完成', latency: '~3-5 cycles' },
    far:  { name: '远端原子', nameEn: 'Far-Atomic', color: '#ff9500', icon: '◎',
             desc: '在本核cache未命中时，通过CHI互连到Home Node/Subordinate Node执行', latency: '~20-100+ cycles' },
    mixed:{ name: '混合自适应', nameEn: 'Mixed', color: '#af52de', icon: '◈',
             desc: '快速路径使用近端原子(L1命中)，慢速路径使用远端原子(L1未命中)', latency: '可变' }
  },

  categories: [
    { id: 'spinlock', name: '自旋锁类', icon: '⟳', color: '#2a71ff' },
    { id: 'blocking', name: '阻塞锁类', icon: '◧', color: '#ff008a' },
    { id: 'atomic', name: '原子操作类', icon: '⚛', color: '#00d1ff' },
    { id: 'hierarchical', name: '层次化锁', icon: '◈', color: '#ff9500' },
    { id: 'barrier', name: '同步屏障类', icon: '║', color: '#34c759' },
    { id: 'optimization', name: '优化技术', icon: '⚡', color: '#af52de' }
  ],

  locks: [
    // ========== 自旋锁类 ==========
    {
      id: 'simple-spinlock',
      name: '简单自旋锁',
      nameEn: 'Simple Spinlock',
      category: 'spinlock',
      keywords: ['Spinlock', 'LDXR', 'STXR', '忙等待'],
      principle: '忙等待循环检查锁状态，持续占用CPU直到获取锁。',
      armImpl: '使用 LDXR/STXR (Load-Exclusive/Store-Exclusive) 或 ARMv8.1+ LSE原子指令',
      pseudocode: `function spin_lock(lock):
    while true:
        // 原子尝试获取锁
        if atomic_exchange(lock, 0, 1) == 0:
            memory_barrier(acquire)
            return

function spin_unlock(lock):
    memory_barrier(release)
    atomic_store(lock, 0)`,
      performancePoints: [
        '缓存行 bouncing - 所有CPU核心竞争同一内存位置',
        '总线锁定开销 - 原子操作需要总线同步',
        '功耗问题 - 持续自旋消耗CPU周期'
      ],
      scenarios: ['极短临界区', '中断上下文', '不可睡眠环境'],
      recommendations: '临界区 < 100 cycles 时使用',
      metrics: { latency: 5, throughput: 3, fairness: 2, power: 1, scalability: 2 },
      detailedPrinciple: `简单自旋锁是最基础的锁实现之一，其核心思想是"忙等待"（Busy Waiting）。

当一个线程尝试获取锁时，它会不断地检查锁的状态。如果锁被占用，线程会一直循环等待（自旋），直到锁变为可用状态。这种机制完全在用户态完成，不需要内核介入。

在ARMv8架构上，自旋锁通常使用LDXR（Load-Exclusive）和STXR（Store-Exclusive）指令对来实现原子操作。LDXR会标记对内存位置的独占访问，STXR只有在独占标记仍然有效时才能成功写入。

ARMv8.1-A引入了LSE（Large System Extension）原子指令集，如CASAL（Compare-and-Swap Acquire-Release），可以更高效地实现原子操作，减少总线流量。`,
      useCases: [
        { scenario: '中断处理程序', description: '在中断上下文中保护共享数据。中断处理程序不能睡眠，因此必须使用自旋锁而非阻塞锁。' },
        { scenario: '内核态短临界区', description: '保护只需几十个CPU周期的临界区，如修改链表指针或更新计数器。' },
        { scenario: 'NMI上下文', description: '不可屏蔽中断（NMI）上下文中，连自旋锁都需要特殊处理（raw_spinlock）。' }
      ],
      prosAndCons: {
        pros: ['实现简单，代码量小', '无上下文切换开销', '延迟可预测（无竞争时）', '可在中断上下文使用'],
        cons: ['浪费CPU周期（忙等待）', '高竞争下性能急剧下降', '功耗高（持续运行CPU）', '可能导致优先级反转']
      },
      warnings: ['不要在持有自旋锁时调用可能睡眠的函数', '临界区应尽可能短（建议 < 100 cycles）', '避免在自旋锁临界区内调用其他可能获取锁的函数（死锁风险）'],
      hardwareTendency: {
        type: 'far',
        tendency: 85,
        description: '每次CAS导致缓存行跨核颠簸，LDXR/STXR始终移动数据，大量互连流量',
        armDetails: {
          instructions: 'LDXR/STXR 跨核独占访问',
          mesiState: 'Shared → Unique 状态转换',
          chiTransactions: 'CHI AtomicStore 或 ReadShared + CleanUnique'
        },
        performanceImpact: {
          uncontended: '~15-25 cycles',
          contended: '~100-300 cycles',
          numaCrossNode: '~500-2000 cycles'
        }
      },
      cmhHints: {
        shuh: {
          enabled: true,
          description: '向内存系统提示即将发生共享更新操作。如果下一条指令产生显式内存效应，内存系统可为共享更新做准备。',
          codeLocation: '在进入高竞争循环前，SHUH指令提示硬件即将进行共享锁变量更新',
          benefit: '理论收益：在高竞争场景下可能减少5-15%的自旋时间（未经验证）',
          warnings: [
            '仅在确认存在高竞争（多核心同时竞争）时使用',
            '需要在实际硬件上测试验证效果',
            '某些处理器可能忽略这些提示，增加指令开销',
            '不适用于单线程或低竞争场景'
          ],
          experimental: true
        },
        stcph: {
          enabled: true,
          description: '向内存系统提示优先处理并发存储操作。如果下一条指令是内存访问，内存系统应优先处理。',
          codeLocation: '在释放锁前，STCPH指令提示硬件这个存储操作需要优先处理',
          benefit: '理论收益：可能减少其他核心的等待延迟（未经验证）',
          warnings: [
            'STCPH是STEOR/STEORL（原子异或）的别名',
            '仅在确认存在高竞争时使用',
            '需要实测验证收益',
            '某些处理器可能忽略这些提示'
          ],
          experimental: true
        }
      }
    },
    {
      id: 'ttas',
      name: 'TTAS锁',
      nameEn: 'Test-Then-Test-And-Set',
      category: 'spinlock',
      keywords: ['TTAS', 'Spinlock', '缓存优化', 'Test-And-Set'],
      principle: '先读取锁状态，仅在锁空闲时才执行原子操作。减少缓存一致性流量。',
      armImpl: '普通读取测试 + LDXR/STXR 原子获取',
      pseudocode: `function ttas_lock(lock):
    while true:
        // 第一阶段：普通读取测试
        if load(lock) == 0:
            // 第二阶段：原子获取
            if atomic_cas(lock, 0, 1):
                memory_barrier(acquire)
                return`,
      performancePoints: [
        '减少缓存一致性流量 - 先读后原子操作',
        '高竞争下仍有效率问题 - 多核同时发现锁空闲'
      ],
      scenarios: ['中等竞争场景', '需要减少总线流量'],
      recommendations: '竞争程度中等时优先选择',
      metrics: { latency: 4, throughput: 4, fairness: 2, power: 2, scalability: 3 },
      detailedPrinciple: `TTAS（Test-Then-Test-And-Set）锁是对简单自旋锁的优化，采用两阶段检查机制。

第一阶段使用普通读取指令检查锁状态，这不会产生缓存一致性流量。只有当发现锁空闲时，才执行第二阶段的原子CAS操作来获取锁。

简单自旋锁每次迭代都执行原子操作，会导致大量的缓存一致性流量（缓存行在多个核心间传递）。TTAS通过先读后原子操作的方式，大幅减少了这种流量。

然而，在高竞争场景下，当多个核心同时发现锁空闲并尝试CAS操作时，仍会产生竞争问题。`,
      useCases: [
        { scenario: '中等竞争的短临界区', description: '当竞争程度中等时，TTAS在减少总线流量和获取锁速度之间取得了良好平衡。' },
        { scenario: 'NUMA系统优化', description: '在NUMA架构中，减少跨节点的缓存一致性流量对性能至关重要。' },
        { scenario: '缓存敏感场景', description: '当需要减少内存总线压力时，TTAS比简单自旋锁更友好。' }
      ],
      prosAndCons: {
        pros: ['减少缓存一致性流量', '无竞争时性能优秀', '实现相对简单', '比简单自旋锁更高效'],
        cons: ['高竞争下仍有问题', '不能保证公平性', '仍存在CPU空转']
      },
      warnings: ['仍然不能保证公平性', '高竞争时性能会下降', '不适合极长临界区'],
      hardwareTendency: {
        type: 'near',
        tendency: 30,
        description: '读阶段L1本地命中(MESI Shared状态)，仅CAS阶段触发远端操作',
        armDetails: {
          instructions: 'LDR (普通读取) + LDXR/STXR (仅CAS阶段)',
          mesiState: 'Shared (读阶段L1命中) → Unique/Exclusive (CAS阶段)',
          chiTransactions: 'ReadShared (仅CAS时需要CHI事务)'
        },
        performanceImpact: {
          uncontended: '~3-8 cycles',
          contended: '~50-150 cycles',
          numaCrossNode: '~200-800 cycles'
        }
      }
    },
    {
      id: 'ticket-spinlock',
      name: 'Ticket自旋锁',
      nameEn: 'Ticket Spinlock',
      category: 'spinlock',
      keywords: ['Ticket', 'Spinlock', 'Fairness', 'FIFO', '公平性'],
      principle: 'FIFO公平性保证，类似银行叫号系统。每个等待者获取票号，按顺序获取锁。',
      armImpl: '使用 atomic_fetch_add 获取票号，WFE低功耗等待',
      pseudocode: `struct ticket_lock:
    next_ticket = 0      // 下一个可用票号
    now_serving = 0      // 当前服务号

function lock(l):
    my_ticket = atomic_fetch_add(l.next_ticket, 1)
    while l.now_serving != my_ticket:
        wait_for_event()  // ARM WFE低功耗等待

function unlock(l):
    atomic_fetch_add(l.now_serving, 1)
    broadcast_event()    // ARM SEV唤醒等待者`,
      performancePoints: [
        '所有等待者轮询同一owner变量 → 缓存行颠簸',
        '公平性带来额外开销 - 需要维护票号计数器'
      ],
      scenarios: ['需要公平性保证', '中等竞争场景'],
      recommendations: '需要严格FIFO顺序时使用',
      metrics: { latency: 4, throughput: 3, fairness: 5, power: 3, scalability: 3 },
      detailedPrinciple: `Ticket自旋锁采用类似银行叫号系统的机制，确保FIFO（先进先出）公平性。

锁维护两个计数器：next_ticket（下一个可用票号）和now_serving（当前服务号）。每个请求锁的线程先原子地获取一个票号，然后等待自己的票号被叫到。

由于票号是递增分配的，且获取锁的顺序严格按照票号顺序，因此不会出现饥饿现象。每个等待者最终都能获得锁。

在ARM架构上，可以使用WFE（Wait For Event）指令进入低功耗状态，配合SEV（Send Event）指令唤醒等待者，可以显著降低等待时的功耗。`,
      useCases: [
        { scenario: '实时系统', description: '在实时系统中，公平性至关重要。Ticket锁确保所有线程按顺序获取锁，避免饥饿。' },
        { scenario: '避免线程饥饿', description: '当系统需要保证所有线程都能公平地获取资源时，Ticket锁的FIFO特性非常关键。' },
        { scenario: '确定性延迟要求', description: '当需要可预测的锁获取延迟时，Ticket锁的有序性提供了保证。' }
      ],
      prosAndCons: {
        pros: ['保证FIFO公平性', '避免线程饥饿', '低功耗等待（WFE）', '延迟可预测'],
        cons: ['所有等待者轮询同一变量', '缓存行颠簸仍存在', '票号可能溢出（需要特殊处理）']
      },
      warnings: ['票号变量可能溢出，需要使用足够大的整数类型', '所有等待者仍轮询同一变量，存在缓存行颠簸', '不适合极高竞争场景（考虑MCS）'],
      hardwareTendency: {
        type: 'mixed',
        tendency: 55,
        description: '本地ticket检查为近端读，但serving counter更新和WFE唤醒涉及远端同步',
        armDetails: {
          instructions: 'LDADDAL (原子递增), LDR + WFE (等待), ADD + STLR + SEV (释放)',
          mesiState: 'Shared (等待时读L1命中) → Unique (更新ticket时)',
          chiTransactions: 'ReadShared (等待), AtomicStore (更新counter)'
        },
        performanceImpact: {
          uncontended: '~10-20 cycles',
          contended: '~80-200 cycles',
          numaCrossNode: '~300-1000 cycles'
        }
      },
      cmhHints: {
        shuh: {
          enabled: true,
          description: '向内存系统提示即将发生共享更新操作。多个核心同时原子递增next_ticket，产生缓存行竞争。',
          codeLocation: '在原子递增前，SHUH指令提示硬件即将更新共享计数器',
          benefit: '理论收益：SHUH可能优化这个高频原子递增操作的缓存行为（未经验证）',
          warnings: [
            '只有在多核心同时竞争时才有效',
            'WFE/SEV配合使用更重要，CMH hints是额外优化',
            '需要实测验证收益'
          ],
          experimental: true
        },
        stcph: {
          enabled: true,
          description: '向内存系统提示优先处理并发存储操作。now_serving被所有等待者轮询。',
          codeLocation: '在更新now_serving前，STCPH指令提示硬件这是关键更新',
          benefit: '理论收益：可能减少其他核心的等待延迟（未经验证）',
          warnings: [
            '只有在多核心同时竞争时才有效',
            '需要实测验证收益',
            '某些处理器可能忽略这些提示'
          ],
          experimental: true
        }
      }
    },
    {
      id: 'mcs-spinlock',
      name: 'MCS队列自旋锁',
      nameEn: 'MCS Queue Spinlock',
      category: 'spinlock',
      keywords: ['MCS', 'Queue', 'Spinlock', 'ih_queued_spinlock', 'NUMA'],
      principle: '每个等待者在本地变量上自旋，形成链表队列。消除缓存行颠簸。',
      armImpl: '链表操作 + 本地变量自旋',
      pseudocode: `struct mcs_node:
    next = null
    locked = 1

struct mcs_lock:
    tail = null

function lock(l, node):
    node.next = null
    node.locked = 1
    predecessor = atomic_swap(l.tail, node)
    if predecessor != null:
        predecessor.next = node
        while node.locked:  // 自旋本地变量
            wait()

function unlock(l, node):
    if node.next == null:
        if atomic_cas(l.tail, node, null):
            return
        while node.next == null:
            wait()
    node.next.locked = 0`,
      performancePoints: [
        '每个CPU自旋本地变量 - 消除缓存行颠簸',
        '极佳的NUMA可扩展性',
        '内存开销较大 - 每CPU需要节点'
      ],
      scenarios: ['高竞争场景', '多核/NUMA系统'],
      recommendations: '高竞争、多核系统首选',
      metrics: { latency: 3, throughput: 5, fairness: 5, power: 4, scalability: 5 },
      detailedPrinciple: `MCS（Mellor-Crummey and Scott）队列自旋锁是一种可扩展的公平锁实现。

每个等待者在自己的本地节点上自旋，而不是在全局变量上自旋。等待者通过链表连接，形成一个队列。这彻底消除了缓存行颠簸问题。

在NUMA系统中，不同CPU节点的内存访问延迟不同。MCS锁让每个CPU在自己的本地变量上自旋，避免了跨节点的缓存一致性流量。

Linux内核的ih_queued_spinlock就是基于MCS设计的，是多核系统中的关键同步原语。`,
      useCases: [
        { scenario: '高竞争多核系统', description: '当大量CPU核心竞争同一把锁时，MCS的本地自旋特性可以显著提升性能。' },
        { scenario: 'NUMA架构服务器', description: '在多插槽服务器中，MCS避免了跨插槽的缓存行颠簸，是Linux内核的首选。' },
        { scenario: '数据库系统', description: '高并发的数据库系统常用MCS类锁来保护关键数据结构。' }
      ],
      prosAndCons: {
        pros: ['本地自旋，无缓存颠簸', '极佳的NUMA可扩展性', '保证公平性（FIFO）', '高竞争下性能优异'],
        cons: ['实现复杂', '每个CPU需要节点内存', '低竞争时有额外开销', '代码体积较大']
      },
      warnings: ['必须确保节点内存在锁持有期间保持有效', '每CPU需要预分配节点', '低竞争时额外开销可能不值得'],
      hardwareTendency: {
        type: 'near',
        tendency: 15,
        description: '每个等待者在本地节点变量自旋，Near-Atomic理想情况，极少互连流量',
        armDetails: {
          instructions: 'LDR (本地自旋), STLR (传递锁)',
          mesiState: 'Unique/Exclusive - 本地L1缓存独占状态',
          chiTransactions: '无需CHI事务 - 本地执行'
        },
        performanceImpact: {
          uncontended: '~5-15 cycles',
          contended: '~20-60 cycles',
          numaCrossNode: '~100-300 cycles'
        }
      }
    },
    {
      id: 'array-spinlock',
      name: '数组队列自旋锁',
      nameEn: 'Array-Based Queued Spinlock',
      category: 'spinlock',
      keywords: ['Array', 'Queue', 'Spinlock', '缓存局部性'],
      principle: '使用固定数组替代链表，预分配节点。更好的缓存局部性。',
      armImpl: '数组索引 + 原子操作',
      pseudocode: `struct array_lock:
    tickets[N]  // N = CPU数量
    queue_head = 0
    queue_tail = 0

function lock(l):
    my_slot = atomic_fetch_add(l.queue_tail, 1) % N
    while l.tickets[my_slot] == 0:
        wait()

function unlock(l):
    next_slot = (my_slot + 1) % N
    l.tickets[next_slot] = 1`,
      performancePoints: [
        '更好的缓存局部性 - 预分配数组',
        '避免动态内存分配',
        '固定大小限制灵活性'
      ],
      scenarios: ['CPU数量固定', '需要缓存友好'],
      recommendations: 'CPU数量已知且固定的系统',
      metrics: { latency: 3, throughput: 4, fairness: 5, power: 4, scalability: 4 },
      hardwareTendency: {
        type: 'near',
        tendency: 20,
        description: '在本地预分配数组槽位自旋，缓存行对齐减少冲突，Near-Atomic友好',
        armDetails: {
          instructions: 'LDR (本地槽位), STLR (传递)',
          mesiState: 'Unique/Exclusive - 本地L1缓存独占状态',
          chiTransactions: '极少的Data传递 (仅锁传递时)'
        },
        performanceImpact: {
          uncontended: '~5-12 cycles',
          contended: '~30-80 cycles',
          numaCrossNode: '~150-500 cycles'
        }
      }
    },

    // ========== 阻塞锁类 ==========
    {
      id: 'mutex',
      name: '互斥锁',
      nameEn: 'Mutex',
      category: 'blocking',
      keywords: ['Mutex', '互斥锁', '阻塞', '上下文切换'],
      principle: '竞争时让出CPU，内核调度其他线程。适合较长临界区。',
      armImpl: '用户态快速路径 + 内核态慢速路径',
      pseudocode: `function mutex_lock(m):
    // 快速路径：无竞争
    if atomic_cas(m.state, 0, 1):
        return
    // 慢速路径：让出CPU
    while true:
        if atomic_cas(m.state, 0, 1):
            return
        kernel_wait(m)  // 系统调用，内核调度

function mutex_unlock(m):
    atomic_store(m.state, 0)
    kernel_wake(m)  // 唤醒等待者`,
      performancePoints: [
        '上下文切换开销 (µs级别)',
        '内核-用户态切换开销'
      ],
      scenarios: ['临界区较长', '可能阻塞', '低竞争'],
      recommendations: '临界区 > 1000 cycles 或可能阻塞时使用',
      metrics: { latency: 2, throughput: 4, fairness: 4, power: 5, scalability: 4 },
      detailedPrinciple: `互斥锁（Mutex）是一种阻塞锁，当竞争发生时会让出CPU，由内核调度其他线程。

与自旋锁不同，mutex在获取失败时不会忙等待，而是将当前线程加入等待队列，然后触发上下文切换让出CPU。当锁释放时，内核会唤醒等待队列中的一个线程。

mutex的实现需要与内核调度器紧密配合。在Linux中，这通过futex系统调用实现，内核负责管理等待队列和线程唤醒。

实时系统中常用带优先级继承的mutex（PI mutex），防止优先级反转问题。当高优先级线程等待低优先级线程持有的锁时，低优先级线程会临时继承高优先级。`,
      useCases: [
        { scenario: '长临界区', description: '当临界区可能执行较长时间（如文件I/O、网络操作）时，使用mutex让其他线程有机会运行。' },
        { scenario: '可能阻塞的操作', description: '如果临界区内可能发生阻塞（如等待I/O、内存分配），必须使用mutex而非自旋锁。' },
        { scenario: '用户态应用', description: '大多数用户态多线程程序使用pthread_mutex，底层就是mutex实现。' }
      ],
      prosAndCons: {
        pros: ['不浪费CPU（阻塞等待）', '适合长临界区', '支持优先级继承', '低竞争时效率高'],
        cons: ['上下文切换开销大（µs级）', '内核-用户态切换开销', '不适合短临界区', '不能在中断上下文使用']
      },
      warnings: ['不能在中断上下文使用', '临界区过长会影响系统响应性', '注意优先级反转问题（使用PI mutex）'],
      hardwareTendency: {
        type: 'mixed',
        tendency: 50,
        description: '快速路径近端CAS用户态，慢速路径内核futex系统调用远端',
        armDetails: {
          instructions: 'CASAL (快速路径), SVC #0 + futex (慢速路径)',
          mesiState: 'Unique/Exclusive (快速路径L1命中) → 远端 (慢速路径L1未命中)',
          chiTransactions: '快速路径: 本地Data; 慢速路径: ReadShared/Invalid全系统'
        },
        performanceImpact: {
          uncontended: '~10-25 cycles',
          contended: '~1000-5000 cycles (上下文切换)',
          numaCrossNode: '~5000-20000 cycles'
        }
      }
    },
    {
      id: 'futex',
      name: 'Futex',
      nameEn: 'Fast Userspace Mutex',
      category: 'blocking',
      keywords: ['futex', 'Fast Userspace', '混合锁', 'Linux'],
      principle: '无竞争时完全在用户态完成，竞争时调用内核。最佳通用选择。',
      armImpl: '用户态原子操作 + futex(2) 系统调用',
      pseudocode: `function futex_lock(futex):
    // 快速路径：无竞争时完全用户态
    if atomic_exchange(futex, 1) == 0:
        return  // 成功获取锁

    // 慢速路径：需要内核介入
    while true:
        expected = 2  // 2表示有等待者
        if atomic_cas(futex, 0, 2):
            return
        atomic_store(futex, 2)
        futex_wait(futex, 2)  // 系统调用

function futex_unlock(futex):
    old = atomic_exchange(futex, 0)
    if old == 2:  // 有等待者
        futex_wake(futex, 1)  // 唤醒一个`,
      performancePoints: [
        '无竞争时极快 (~10-20 cycles)',
        '竞争时系统调用开销'
      ],
      scenarios: ['通用场景', '竞争不确定'],
      recommendations: '竞争不确定时的首选',
      metrics: { latency: 4, throughput: 4, fairness: 3, power: 4, scalability: 4 },
      detailedPrinciple: `Futex（Fast Userspace Mutex）是Linux特有的高效同步原语，结合了自旋锁和阻塞锁的优点。

无竞争时完全在用户态完成，使用原子操作快速获取锁（约10-20 cycles）。只有在竞争发生时才调用futex(2)系统调用进入内核态。

状态值含义：
- 0: 锁空闲
- 1: 锁被持有，无等待者
- 2: 锁被持有，有等待者

在低竞争场景下性能接近自旋锁，在高竞争场景下能避免CPU空转。glibc的pthread_mutex底层就是基于futex实现的，因此是通用场景的最佳选择。`,
      useCases: [
        { scenario: '通用多线程程序', description: 'Futex是pthread_mutex的底层实现，适用于大多数用户态同步场景。' },
        { scenario: '竞争不确定的场景', description: '当无法预知锁竞争程度时，futex的自适应特性使其成为最安全的选择。' },
        { scenario: '高性能服务器', description: '现代高性能服务器应用广泛使用基于futex的同步原语。' }
      ],
      prosAndCons: {
        pros: ['无竞争时极快（~10-20 cycles）', '自适应竞争程度', '通用性最佳', 'pthread_mutex底层实现'],
        cons: ['Linux特有（非标准）', '竞争时系统调用开销', '实现细节复杂']
      },
      warnings: ['是Linux特有接口，移植性考虑', '状态值管理复杂，容易出错', '直接使用futex(2)需要小心处理边界情况'],
      hardwareTendency: {
        type: 'mixed',
        tendency: 45,
        description: '无竞争纯用户态CAS近端，竞争时futex系统调用远端内核',
        armDetails: {
          instructions: 'CASAL (用户态快速路径), SVC #0 + futex(2) (慢速路径)',
          mesiState: 'Unique/Exclusive (快速路径L1命中) → 远端 (内核futex)',
          chiTransactions: '快速: 本地Data; 慢速: ReadShared/Invalid全系统'
        },
        performanceImpact: {
          uncontended: '~10-20 cycles',
          contended: '~800-3000 cycles',
          numaCrossNode: '~3000-15000 cycles'
        }
      }
    },

    // ========== 原子操作类 ==========
    {
      id: 'cas-lock',
      name: 'CAS锁',
      nameEn: 'Compare-And-Swap Lock',
      category: 'atomic',
      keywords: ['CAS', 'Compare-And-Swap', '原子操作', '无锁'],
      principle: '基于CAS原语实现的轻量锁。无锁数据结构的基础。',
      armImpl: 'LDXR/STXR 或 CASAL 指令',
      pseudocode: `function cas_lock(lock):
    while true:
        if atomic_cas(lock, 0, 1):  // 原子比较交换
            memory_barrier(acquire)
            return

function cas_unlock(lock):
    memory_barrier(release)
    atomic_store(lock, 0)

// 无锁栈示例
function push(stack, node):
    while true:
        old_head = stack.head
        node.next = old_head
        if atomic_cas(stack.head, old_head, node):
            return`,
      performancePoints: [
        'ABA问题 - 需要额外处理',
        '内存顺序开销 - 需要正确设置屏障'
      ],
      scenarios: ['无锁数据结构', '轻量同步', '单次操作'],
      recommendations: '简单原子操作优先使用',
      metrics: { latency: 5, throughput: 3, fairness: 1, power: 2, scalability: 3 },
      hardwareTendency: {
        type: 'mixed',
        tendency: 60,
        description: '无竞争时近端CAS，竞争时远端缓存行颠簸，偏向远端',
        armDetails: {
          instructions: 'LDXR/STXR 或 CASAL',
          mesiState: 'Unique/Exclusive (无竞争L1命中) → Shared/Invalid (竞争L1未命中)',
          chiTransactions: 'Compare, Data (竞争时需要CHI事务)'
        },
        performanceImpact: {
          uncontended: '~5-15 cycles',
          contended: '~80-250 cycles',
          numaCrossNode: '~400-1500 cycles'
        }
      }
    },

    // ========== 层次化锁 ==========
    {
      id: 'global-lock',
      name: '全局锁',
      nameEn: 'Global Lock',
      category: 'hierarchical',
      keywords: ['全局锁', 'Global', '粗粒度', 'Big Lock'],
      principle: '单一锁保护所有资源。简单但扩展性差。',
      armImpl: '任意锁类型 + 全局作用域',
      pseudocode: `// 全局单一锁
global_lock system_lock

function access_resource(id):
    lock(system_lock)
    // 访问任意资源
    resource = resources[id]
    process(resource)
    unlock(system_lock)`,
      performancePoints: [
        '扩展性瓶颈 - 所有操作串行化',
        '高竞争 - 任何操作都需要获取锁'
      ],
      scenarios: ['单核系统', '极低竞争', '初始化阶段'],
      recommendations: '仅在简单场景或过渡期使用',
      metrics: { latency: 3, throughput: 1, fairness: 3, power: 3, scalability: 1 },
      hardwareTendency: {
        type: 'far',
        tendency: 90,
        description: '所有核心竞争同一缓存行，持续互连流量，典型Far-Atomic',
        armDetails: {
          instructions: 'LDXR/STXR 跨核独占访问',
          mesiState: 'Shared → Unique 状态转换',
          chiTransactions: 'CHI AtomicStore 或 ReadShared + CleanUnique (高频)'
        },
        performanceImpact: {
          uncontended: '~15-30 cycles',
          contended: '~200-800 cycles',
          numaCrossNode: '~1000-5000 cycles'
        }
      }
    },
    {
      id: 'local-lock',
      name: '局部锁',
      nameEn: 'Local Lock',
      category: 'hierarchical',
      keywords: ['局部锁', 'Local', '细粒度', 'Per-CPU'],
      principle: '每个数据结构独立锁。细粒度并行。',
      armImpl: 'Per-CPU变量 + 独立锁实例',
      pseudocode: `// 每个资源独立锁
struct resource:
    data
    lock

// Per-CPU锁
per_cpu_locks[N]  // N = CPU数量

function access_local(id):
    cpu_id = get_cpu_id()
    lock(per_cpu_locks[cpu_id])
    // 访问本地资源
    process(local_data[id])
    unlock(per_cpu_locks[cpu_id])`,
      performancePoints: [
        '细粒度并行 - 减少锁竞争',
        '需要 careful 设计避免死锁'
      ],
      scenarios: ['分片数据', 'per-CPU变量', '高并发'],
      recommendations: '数据可分片时优先使用',
      metrics: { latency: 4, throughput: 5, fairness: 3, power: 4, scalability: 5 },
      hardwareTendency: {
        type: 'near',
        tendency: 20,
        description: 'Per-CPU独立锁，低竞争L1本地，Near-Atomic理想情况',
        armDetails: {
          instructions: 'LDR/STR (本地CPU数据), LDXR/STXR (低竞争)',
          mesiState: 'Unique/Exclusive - 本地L1缓存独占状态',
          chiTransactions: '极少，仅本地缓存 (几乎无需CHI事务)'
        },
        performanceImpact: {
          uncontended: '~3-8 cycles',
          contended: '~15-40 cycles',
          numaCrossNode: '~50-200 cycles'
        }
      }
    },
    {
      id: 'hierarchical-lock',
      name: '层次锁',
      nameEn: 'Hierarchical Lock',
      category: 'hierarchical',
      keywords: ['层次锁', 'Hierarchical', '分层', '锁升级'],
      principle: '多级锁，按资源层次组织。如数据库行锁→表锁→全局锁。',
      armImpl: '锁层级管理 + 顺序获取',
      pseudocode: `// 数据库层次锁示例
struct lock_hierarchy:
    global_lock      // Level 0
    table_locks[T]   // Level 1
    row_locks[R]     // Level 2

function access_row(table_id, row_id):
    // 按层次顺序获取锁
    lock(global_lock, SHARED)
    lock(table_locks[table_id], SHARED)
    lock(row_locks[row_id], EXCLUSIVE)
    // 访问数据
    process(row_data)
    // 按相反顺序释放
    unlock(row_locks[row_id])
    unlock(table_locks[table_id])
    unlock(global_lock)`,
      performancePoints: [
        '死锁风险 - 需要严格的锁顺序',
        '锁升级开销 - 可能需要升级锁级别'
      ],
      scenarios: ['数据库系统', '文件系统', '层次化资源'],
      recommendations: '需要严格定义锁获取顺序',
      metrics: { latency: 3, throughput: 4, fairness: 4, power: 3, scalability: 4 },
      hardwareTendency: {
        type: 'mixed',
        tendency: 50,
        description: '本地层近端锁(行锁)，全局层远端锁(表锁/全局锁)',
        armDetails: {
          instructions: '多层LDXR/STXR + DMB ISH',
          mesiState: '本地层: Unique (L1命中), 全局层: Shared → Unique (L1未命中)',
          chiTransactions: '本地层: 少, 全局层: ReadShared/Invalid'
        },
        performanceImpact: {
          uncontended: '~10-30 cycles',
          contended: '~100-500 cycles',
          numaCrossNode: '~500-3000 cycles'
        }
      }
    },

    // ========== 同步屏障类 ==========
    {
      id: 'pthread-barrier',
      name: '线程屏障',
      nameEn: 'pthread_barrier',
      category: 'barrier',
      keywords: ['pthread_barrier', '屏障', '同步点', '并行算法'],
      principle: '等待所有线程到达同步点后一起释放。用于并行算法的阶段同步。',
      armImpl: '基于futex或条件变量实现',
      pseudocode: `struct barrier:
    count = 0
    target = N  // 需要等待的线程数
    mutex
    cond

function barrier_wait(b):
    lock(b.mutex)
    b.count += 1
    if b.count == b.target:
        b.count = 0
        cond_broadcast(b.cond)
        unlock(b.mutex)
        return
    while b.count != 0:
        cond_wait(b.cond, b.mutex)
    unlock(b.mutex)`,
      performancePoints: [
        '所有线程必须到达 - 最慢线程决定延迟',
        '内存屏障开销 - 确保可见性'
      ],
      scenarios: ['并行算法阶段同步', '迭代计算', 'MapReduce'],
      recommendations: '需要所有线程同步时使用',
      metrics: { latency: 2, throughput: 3, fairness: 5, power: 3, scalability: 3 },
      hardwareTendency: {
        type: 'far',
        tendency: 80,
        description: '全对全同步，大量跨核计数器更新和条件变量广播，Far-Atomic主导',
        armDetails: {
          instructions: 'LDADDAL (计数器), DMB ISH + STLR (屏障), SEV (唤醒)',
          mesiState: 'Shared → Unique 状态转换 (计数器颠簸)',
          chiTransactions: 'AtomicStore, Data (全系统广播)'
        },
        performanceImpact: {
          uncontended: '~50-100 cycles',
          contended: '~500-2000 cycles',
          numaCrossNode: '~2000-10000 cycles'
        }
      },
      cmhHints: {
        shuh: {
          enabled: true,
          description: '向内存系统提示即将发生共享更新操作。count计数器被所有线程原子更新。',
          codeLocation: '在原子递增计数器前，SHUH指令提示硬件即将更新共享变量',
          benefit: '理论收益：可能优化计数器的缓存一致性协议（未经验证）',
          warnings: [
            '屏障同步的开销主要在算法层面，CMH hints的收益可能很小',
            '更重要的是优化屏障使用频率和算法设计',
            '实测验证非常必要'
          ],
          experimental: true
        },
        stcph: {
          enabled: true,
          description: '向内存系统提示优先处理并发存储操作。cond_broadcast需要尽快唤醒所有等待线程。',
          codeLocation: '在广播唤醒前，STCPH指令提示硬件这是关键同步操作',
          benefit: '理论收益：可能优先处理这个广播操作（未经验证）',
          warnings: [
            '屏障同步的开销主要在算法层面',
            '需要实测验证收益',
            '某些处理器可能忽略这些提示'
          ],
          experimental: true
        }
      }
    },

    // ========== 优化技术 ==========
    {
      id: 'shared-cache',
      name: '共享缓存感知优化',
      nameEn: 'Shared Cache Optimization',
      category: 'optimization',
      keywords: ['shared cache', '缓存感知', 'big.LITTLE', 'DynamIQ', 'NUMA'],
      principle: '考虑ARM big.LITTLE/DynamIQ架构的缓存层次进行优化。',
      armImpl: '缓存行对齐 + 内存屏障 + LSE指令',
      pseudocode: `// 缓存行对齐 (通常64字节)
struct aligned_lock:
    lock
    padding[63]  // 填充到缓存行大小

// 避免false sharing
struct per_cpu_data:
    value
    padding[...]  // 确保不同CPU数据在不同缓存行

// ARM内存屏障
function acquire_barrier():
    dmb(ish)  // Inner Shareable 数据内存屏障

function release_barrier():
    dmb(ish)

// ARM低功耗等待
function wait_for_lock():
    wfe()  // Wait For Event，低功耗状态`,
      performancePoints: [
        '锁变量对齐到缓存行 - 避免false sharing',
        '小核集群使用独立锁 - 减少跨簇竞争',
        'NUMA感知锁分配 - 就近访问'
      ],
      scenarios: ['ARM big.LITTLE系统', 'NUMA服务器', '高性能计算'],
      recommendations: '多簇/多插槽系统必须考虑',
      metrics: { latency: 4, throughput: 5, fairness: 3, power: 5, scalability: 5 },
      hardwareTendency: {
        type: 'near',
        tendency: 10,
        description: '专门优化缓存局部性，Near-Atomic极致优化技术',
        armDetails: {
          instructions: 'DC ZVA (缓存行对齐), DMB ISH, WFE',
          mesiState: 'Unique/Exclusive - 本地L1缓存独占状态 (优化目标)',
          chiTransactions: '最小化 CHI事务'
        },
        performanceImpact: {
          uncontended: '~2-5 cycles',
          contended: '~10-30 cycles',
          numaCrossNode: '~30-100 cycles'
        }
      }
    },
    {
      id: 'lockharmer',
      name: 'LockHarmer分析工具',
      nameEn: 'LockHarmer Profiler',
      category: 'optimization',
      keywords: ['LockHarmer', '性能分析', '锁竞争', 'profiling'],
      principle: '锁性能分析工具，用于识别锁竞争和性能瓶颈。',
      armImpl: '性能计数器 + 采样分析',
      pseudocode: `// 关键分析指标
struct lock_stats:
    wait_time      // 等待时间
    hold_time      // 持有时间
    contention_rate  // 竞争程度
    cache_misses   // 缓存未命中率

function profile_lock(lock, stats):
    start = timestamp()
    lock(lock)
    acquire_time = timestamp()
    stats.wait_time += acquire_time - start

    // 临界区操作
    do_critical_section()

    end = timestamp()
    stats.hold_time += end - acquire_time
    unlock(lock)

// 分析报告
function analyze_contention(stats):
    if stats.wait_time / stats.hold_time > 2.0:
        report("高竞争锁，考虑优化")`,
      performancePoints: [
        '等待时间 (wait time) - 竞争程度指标',
        '持有时间 (hold time) - 临界区长度',
        '竞争程度 (contention rate) - 系统瓶颈',
        '缓存未命中率 - 内存访问效率'
      ],
      scenarios: ['性能调优', '瓶颈分析', '系统优化'],
      recommendations: '定期分析锁性能',
      metrics: { latency: 3, throughput: 3, fairness: 3, power: 3, scalability: 3 },
      hardwareTendency: {
        type: 'mixed',
        tendency: 50,
        description: '分析工具，观察两种原子倾向的行为特征',
        armDetails: {
          instructions: 'PMU counters (L1D cache refill, bus access)',
          mesiState: '观察所有状态转换 (Unique ↔ Shared)',
          chiTransactions: '监控全部CHI事务类型'
        },
        performanceImpact: {
          uncontended: 'N/A (分析工具)',
          contended: 'N/A (分析工具)',
          numaCrossNode: 'N/A (分析工具)'
        }
      }
    }
  ],

  // Barrier scenarios for deep analysis
  barrierScenarios: [
    {
      id: 'mapreduce-spark',
      name: 'MapReduce / Spark',
      icon: '⚡',
      color: '#ff9500',
      category: '大数据处理',
      barrierFrequency: 'medium',
      optimalAtomicMode: 'mixed',
      description: 'MapReduce和Spark等大数据框架中，Barrier用于阶段间同步。Map阶段完成后，所有任务必须在Barrier处等待，直到Shuffle阶段准备就绪。',
      barrierRole: '确保Map阶段所有分区数据就绪后才开始Shuffle传输，防止数据不一致',
      phases: [
        { name: 'Map', desc: '各节点独立执行Map任务', barrierType: 'implicit' },
        { name: 'Shuffle', desc: '跨节点数据重分区', barrierType: 'explicit' },
        { name: 'Reduce', desc: '汇总计算最终结果', barrierType: 'implicit' }
      ],
      hardwareAnalysis: {
        nearAtomicImpact: '单个Executor在其L1缓存中以Unique状态持有局部计数器时，本地检查可达到~3-5 cycles的Near-Atomic延迟。但在全对全同步中，只有最后到达的线程触发计数器更新能保持Unique状态。',
        farAtomicImpact: 'Shuffle阶段的Barrier需要多个跨节点Executor原子更新同一全局计数器。L1未命中时通过CHI互连到Home Node执行，Shared状态转换导致~20-100+ cycles的Far-Atomic延迟，是跨节点Barrier的主要开销。',
        cacheTopology: {
          sameCluster: '~300-800ns (同集群L1命中率较高)',
          crossCluster: '~1.5-5us (跨集群Shared状态转换)',
          crossNuma: '~5-20us (跨NUMA Home Node访问)'
        }
      },
      armPerformance: {
        graviton2: { cores: 64, barrierLatency: '~2.5us', throughput: '25M barriers/sec' },
        graviton3: { cores: 64, barrierLatency: '~1.8us', throughput: '35M barriers/sec' },
        graviton4: { cores: 96, barrierLatency: '~1.2us', throughput: '80M barriers/sec' }
      },
      codeExample: `// Apache Spark Barrier同步伪代码
RDD.mapPartitions { partition =>
  val result = process(partition)
  barrier.wait()  // 所有分区完成Map
  result
}
// Shuffle阶段自动触发`,
      realSoftware: ['Apache Spark', 'Hadoop MapReduce', 'Flink'],
      concurrencyAnalysis: {
        typicalDeployments: {
          small: {
            cores: '10-20核',
            description: '单机开发/测试环境',
            barrierBehavior: 'Near-Atomic主导，L1命中率高'
          },
          medium: {
            cores: '500-5000核',
            description: '生产环境集群',
            barrierBehavior: '混合模式，跨节点Far-Atomic'
          },
          large: {
            cores: '10000+核',
            description: '大规模集群/超算',
            barrierBehavior: 'Far-Atomic主导，延迟累积'
          }
        },
        performanceByCores: [
          { cores: 16, latency: '500ns', throughput: '高', bottleneck: '无' },
          { cores: 64, latency: '1.2us', throughput: '中', bottleneck: '跨集群' },
          { cores: 256, latency: '4us', throughput: '低', bottleneck: '跨NUMA' },
          { cores: 1024, latency: '12us', throughput: '极低', bottleneck: '网络+远端原子' }
        ]
      }
    },
    {
      id: 'mpi-collective',
      name: 'MPI集合通信',
      icon: '🔌',
      color: '#2a71ff',
      category: '高性能计算',
      barrierFrequency: 'high',
      optimalAtomicMode: 'mixed',
      description: 'MPI (Message Passing Interface) 中 MPI_Barrier 是最基础的集合操作。在ARM集群上运行OpenMPI/MPICH时，Barrier的实现直接影响并行应用的扩展性。',
      barrierRole: '确保所有MPI进程到达同步点，用于算法阶段划分和时序保证',
      phases: [
        { name: 'Init', desc: 'MPI_Init初始化通信域', barrierType: 'explicit' },
        { name: 'Compute', desc: '各进程独立计算', barrierType: 'implicit' },
        { name: 'Communicate', desc: 'MPI_Barrier + Allreduce', barrierType: 'explicit' }
      ],
      hardwareAnalysis: {
        nearAtomicImpact: '共享内存MPI（单节点多进程）中，主进程在其L1缓存以Unique状态持有Barrier计数器。其他进程读取时需通过互连，仅最后到达进程的原子更新能享受~3-5 cycles的Near-Atomic延迟。',
        farAtomicImpact: '多节点MPI集群中，每个进程的Barrier操作都需要通过CHI互连。计数器在不同节点间以Shared状态传播，每次L1未命中都触发~20-100+ cycles的Far-Atomic延迟。Allreduce操作加剧了这种跨节点互连流量。',
        cacheTopology: {
          sameCluster: '~1-3us (共享内存L2/L3域)',
          crossCluster: '~5-20us (跨集群CHI互连)',
          crossNuma: '~15-60us (跨NUMA + 网络叠加)'
        }
      },
      armPerformance: {
        graviton2: { cores: 64, barrierLatency: '~3.5us (节点内)', throughput: '280K barriers/sec' },
        graviton3: { cores: 64, barrierLatency: '~2.2us (节点内)', throughput: '450K barriers/sec' },
        graviton4: { cores: 96, barrierLatency: '~1.5us (节点内)', throughput: '640K barriers/sec' }
      },
      codeExample: `// MPI Barrier + Allreduce
MPI_Comm_rank(MPI_COMM_WORLD, &rank);
local_sum = compute_partial(rank);
MPI_Barrier(MPI_COMM_WORLD);
MPI_Allreduce(&local_sum, &global_sum, 1, MPI_DOUBLE, MPI_SUM, MPI_COMM_WORLD);`,
      realSoftware: ['OpenMPI', 'MPICH', 'Intel MPI (ARM版)'],
      concurrencyAnalysis: {
        typicalDeployments: {
          small: {
            cores: '4-16核',
            description: '单节点共享内存MPI',
            barrierBehavior: 'Near-Atomic主导，L2/L3共享域通信'
          },
          medium: {
            cores: '64-256核',
            description: '多节点集群',
            barrierBehavior: '混合模式，节点内Near，节点间Far'
          },
          large: {
            cores: '1000+核',
            description: '超级计算集群',
            barrierBehavior: 'Far-Atomic主导，网络+CHI互连延迟'
          }
        },
        performanceByCores: [
          { cores: 4, latency: '1us', throughput: '高', bottleneck: '无' },
          { cores: 16, latency: '1.5us', throughput: '高', bottleneck: 'L3竞争' },
          { cores: 64, latency: '2.2us', throughput: '中', bottleneck: '跨集群' },
          { cores: 256, latency: '8us', throughput: '低', bottleneck: '跨NUMA+网络' }
        ]
      }
    },
    {
      id: 'deep-learning',
      name: '深度学习训练',
      icon: '🧠',
      color: '#af52de',
      category: 'AI/ML',
      barrierFrequency: 'high',
      optimalAtomicMode: 'far',
      description: '分布式数据并行训练中，每个训练步骤结束时需要Barrier同步所有GPU/TPU worker的梯度。这是训练吞吐量的关键瓶颈。',
      barrierRole: '确保所有worker完成梯度计算后才开始AllReduce同步和参数更新',
      phases: [
        { name: 'Forward', desc: '前向传播计算loss', barrierType: 'implicit' },
        { name: 'Backward', desc: '反向传播计算梯度', barrierType: 'implicit' },
        { name: 'AllReduce', desc: '梯度同步 + 参数更新', barrierType: 'explicit' }
      ],
      hardwareAnalysis: {
        nearAtomicImpact: '单节点内多GPU使用共享内存Barrier时，主GPU的L1缓存以Unique状态持有同步变量。NCCL节点内梯度聚合可利用Near-Atomic优化，~3-5 cycles的本地延迟。但梯度同步本质是全对全操作，Near-Atomic优势有限。',
        farAtomicImpact: '跨节点训练中，梯度Barrier和AllReduce都需要通过CHI互连。梯度张量在不同节点间以Shared状态传输，每次L1未命中触发~20-100+ cycles的Far-Atomic延迟。N个worker的梯度同步产生N×Far-Atomic延迟，是分布式训练扩展性的主瓶颈。',
        cacheTopology: {
          sameCluster: '~0.5-2ms (单节点8卡，GPU内部)',
          crossCluster: '~3-15ms (跨节点CHI + NVLink)',
          crossNuma: '~10-40ms (跨机架InfiniBand + Far-Atomic)'
        }
      },
      armPerformance: {
        graviton2: { cores: 64, barrierLatency: '~1.2ms (GPU同步)', throughput: '850 steps/hr' },
        graviton3: { cores: 64, barrierLatency: '~0.8ms (GPU同步)', throughput: '1250 steps/hr' },
        graviton4: { cores: 96, barrierLatency: '~0.5ms (GPU同步)', throughput: '2000 steps/hr' }
      },
      codeExample: `// PyTorch DistributedDataParallel
model = DDP(model, device_ids=[local_rank])
for batch in dataloader:
    loss = model(batch)       # Forward
    loss.backward()           # Backward
    # DDP自动在backward()中插入Barrier
    optimizer.step()          # 参数更新`,
      realSoftware: ['PyTorch Distributed', 'DeepSpeed', 'TensorFlow DistributionStrategy'],
      concurrencyAnalysis: {
        typicalDeployments: {
          small: {
            cores: '8 GPUs',
            description: '单机多卡训练',
            barrierBehavior: 'Near-Atomic主导，NVLink内部通信'
          },
          medium: {
            cores: '32-128 GPUs',
            description: '中等规模训练集群',
            barrierBehavior: '混合模式，节点内Near，节点间Far'
          },
          large: {
            cores: '1000+ GPUs',
            description: '大规模分布式训练',
            barrierBehavior: 'Far-Atomic主导，梯度AllReduce瓶颈'
          }
        },
        performanceByCores: [
          { cores: 8, latency: '0.5ms', throughput: '高', bottleneck: '无' },
          { cores: 32, latency: '1.5ms', throughput: '中', bottleneck: '跨节点梯度' },
          { cores: 128, latency: '5ms', throughput: '低', bottleneck: 'AllReduce同步' },
          { cores: 512, latency: '20ms', throughput: '极低', bottleneck: '网络+远端原子' }
        ]
      }
    },
    {
      id: 'database-parallel',
      name: '数据库并行查询',
      icon: '🗃️',
      color: '#34c759',
      category: '数据库',
      barrierFrequency: 'medium',
      optimalAtomicMode: 'mixed',
      description: 'PostgreSQL 16和MySQL 8.0支持并行查询执行。多个worker并行扫描不同数据分区，需要在Join和Aggregation阶段同步。',
      barrierRole: '并行Scan完成后同步所有worker，确保Join数据就绪',
      phases: [
        { name: 'Parse', desc: '解析SQL生成计划', barrierType: 'implicit' },
        { name: 'Parallel Scan', desc: '多worker并行扫描分区', barrierType: 'implicit' },
        { name: 'Join/Agg', desc: '同步 + Join + 聚合', barrierType: 'explicit' }
      ],
      hardwareAnalysis: {
        nearAtomicImpact: '同NUMA节点的并行Worker在扫描本地分区时，Buffer Pool页面以Unique状态缓存在各Worker的L1中。Barrier到达检查可在本地完成，~3-5 cycles的Near-Atomic延迟使并行扫描效率极高。',
        farAtomicImpact: '跨NUMA的并行查询需要在Join阶段同步。全局Barrier计数器在不同NUMA节点间以Shared状态传播，L1未命中时通过CHI互连到Home Node执行~20-100+ cycles的Far-Atomic延迟。大表Join时这种延迟被放大，影响查询吞吐量。',
        cacheTopology: {
          sameCluster: '~5-20us (同NUMA L1命中)',
          crossCluster: '~50-200us (跨NUMA Shared状态)',
          crossNuma: '~200-800us (跨Socket远端访问)'
        }
      },
      armPerformance: {
        graviton2: { cores: 64, barrierLatency: '~15us', throughput: '650K QPS' },
        graviton3: { cores: 64, barrierLatency: '~10us', throughput: '1.2M QPS' },
        graviton4: { cores: 96, barrierLatency: '~6us', throughput: '2.5M QPS' }
      },
      codeExample: `-- PostgreSQL 16 并行查询
SET max_parallel_workers = 8;
SET parallel_tuple_cost = 0.01;
SELECT * FROM orders JOIN customers
  ON orders.cust_id = customers.id
-- Query Planner自动插入同步屏障
-- Parallel Seq Scan → Barrier → Hash Join`,
      realSoftware: ['PostgreSQL 16', 'MySQL 8.0', 'ClickHouse'],
      concurrencyAnalysis: {
        typicalDeployments: {
          small: {
            cores: '4-8 workers',
            description: '单机并行查询',
            barrierBehavior: 'Near-Atomic主导，L1缓存命中'
          },
          medium: {
            cores: '16-64 workers',
            description: '生产数据库集群',
            barrierBehavior: '混合模式，跨NUMA Far-Atomic'
          },
          large: {
            cores: '128+ workers',
            description: '大规模分析集群',
            barrierBehavior: 'Far-Atomic主导，Join阶段瓶颈'
          }
        },
        performanceByCores: [
          { cores: 4, latency: '10us', throughput: '高', bottleneck: '无' },
          { cores: 16, latency: '20us', throughput: '高', bottleneck: '轻微竞争' },
          { cores: 64, latency: '80us', throughput: '中', bottleneck: '跨NUMA' },
          { cores: 128, latency: '200us', throughput: '低', bottleneck: '远端原子+锁' }
        ]
      }
    },
    {
      id: 'scientific-solver',
      name: '科学计算迭代求解器',
      icon: '🗜️',
      color: '#00d1ff',
      category: '科学计算',
      barrierFrequency: 'high',
      optimalAtomicMode: 'far',
      description: 'PETSc和OpenFOAM中的Jacobi迭代求解器需要在每次迭代步结束时同步所有计算域。Barrier频率极高（每步一次），对互连延迟极其敏感。',
      barrierRole: '每次迭代步后同步所有进程的边界条件，确保收敛性',
      phases: [
        { name: 'Stencil Compute', desc: '计算网格内部点更新', barrierType: 'implicit' },
        { name: 'Boundary Exchange', desc: '交换边界条件', barrierType: 'explicit' },
        { name: 'Convergence Check', desc: '检查收敛性', barrierType: 'explicit' }
      ],
      hardwareAnalysis: {
        nearAtomicImpact: '子域内部点的Stencil计算完全本地化，访问本地网格数据时L1缓存命中率达到~95%，享受~3-5 cycles的Near-Atomic延迟。但每次迭代的Boundary Exchange Barrier需要跨进程同步，Near-Atomic优势仅限于内部计算阶段。',
        farAtomicImpact: '高频率的Barrier（每迭代步一次）使Far-Atomic延迟线性累积。N次迭代需要N次跨进程Boundary Exchange，每次通过CHI互连产生~20-100+ cycles的Far-Atomic延迟。总迭代时间≈N×(内部计算时间+Far-Atomic Barrier延迟)，在收敛慢的场景中Far-Atomic成为主要瓶颈。',
        cacheTopology: {
          sameCluster: '~0.8-3us (同NUMA子域边界)',
          crossCluster: '~4-15us (跨NUMA CHI互连)',
          crossNuma: '~15-50us (跨Socket多层互连)'
        }
      },
      armPerformance: {
        graviton2: { cores: 64, barrierLatency: '~1.8us', throughput: '550K iterations/sec' },
        graviton3: { cores: 64, barrierLatency: '~1.2us', throughput: '830K iterations/sec' },
        graviton4: { cores: 96, barrierLatency: '~0.8us', throughput: '1.2M iterations/sec' }
      },
      codeExample: `// PETSc Jacobi迭代 (简化)
for (iter = 0; iter < max_iter; iter++) {
    // 各进程计算子域内部
    update_interior(local_grid);
    // 同步边界条件
    MPI_Barrier(MPI_COMM_WORLD);
    exchange_boundaries(local_grid, neighbors);
    // 检查收敛
    MPI_Allreduce(&local_res, &global_res, 1, MPI_DOUBLE, MPI_MAX, comm);
    if (global_res < tolerance) break;
}`,
      realSoftware: ['PETSc', 'OpenFOAM', 'Trilinos'],
      concurrencyAnalysis: {
        typicalDeployments: {
          small: {
            cores: '4-16核',
            description: '工作站/小集群',
            barrierBehavior: 'Near-Atomic主导，边界交换本地化'
          },
          medium: {
            cores: '64-256核',
            description: '部门级集群',
            barrierBehavior: '混合模式，子域内Near，边界Far'
          },
          large: {
            cores: '1000+核',
            description: '超级计算机',
            barrierBehavior: 'Far-Atomic主导，高频Barrier累积'
          }
        },
        performanceByCores: [
          { cores: 4, latency: '0.8us', throughput: '高', bottleneck: '无' },
          { cores: 16, latency: '1.5us', throughput: '高', bottleneck: '边界交换' },
          { cores: 64, latency: '3us', throughput: '中', bottleneck: '跨集群' },
          { cores: 256, latency: '10us', throughput: '低', bottleneck: '跨NUMA+迭代累积' }
        ]
      }
    },
    {
      id: 'vulkan-graphics',
      name: 'Vulkan图形管线',
      icon: '🎨',
      color: '#ff008a',
      category: '图形渲染',
      barrierFrequency: 'high',
      optimalAtomicMode: 'near',
      description: 'Vulkan/Metal图形API中，GPU-CPU管线屏障确保渲染pass之间的资源依赖。在ARM Mali GPU和Apple Silicon上，屏障语义直接映射到缓存刷新指令。',
      barrierRole: '确保前一个渲染Pass的写入对后续Pass可见（内存一致性）',
      phases: [
        { name: 'Vertex', desc: '顶点着色器处理', barrierType: 'implicit' },
        { name: 'Fragment', desc: '片段着色器处理', barrierType: 'implicit' },
        { name: 'vkCmdPipelineBarrier', desc: 'CPU-GPU同步屏障', barrierType: 'explicit' }
      ],
      hardwareAnalysis: {
        nearAtomicImpact: 'GPU内部Tile缓存以Unique状态持有渲染数据，同一Render Pass内的Fragment Shader并行处理享受~3-5 cycles的Near-Atomic延迟。ARM Mali的Tile-based架构优化了局部性，颜色/深度附件在GPU Tile内完成，无需外部互连。',
        farAtomicImpact: 'vkCmdPipelineBarrier需要确保前一Pass的GPU Tile写入对后续Pass可见。跨Pass的资源依赖需要通过CHI互连刷新CPU-GPU缓存，GPU L2→CPU的Shared状态转换产生~20-100+ cycles的Far-Atomic延迟。高帧率应用（120+ FPS）中，每帧的Barrier延迟累积影响响应时间。',
        cacheTopology: {
          sameCluster: '~0.1-0.5us (同GPU Tile内部)',
          crossCluster: '~1-8us (GPU L2 → CPU CHI)',
          crossNuma: '~10-50us (跨设备/网络GPU)'
        }
      },
      armPerformance: {
        graviton2: { cores: 64, barrierLatency: '~1.0us (GPU Barrier)', throughput: '60 FPS' },
        graviton3: { cores: 64, barrierLatency: '~0.6us (GPU Barrier)', throughput: '120 FPS' },
        graviton4: { cores: 96, barrierLatency: '~0.3us (GPU Barrier)', throughput: '240 FPS' }
      },
      codeExample: `// Vulkan Pipeline Barrier
vkCmdBindPipeline(cmdBuf, VK_PIPELINE_BIND_POINT_GRAPHICS, pipeline);
vkCmdDraw(cmdBuf, vertexCount, 1, 0, 0);

// 确保颜色附件写入完成
VkMemoryBarrier barrier = {
    .sType = VK_STRUCTURE_TYPE_MEMORY_BARRIER,
    .srcAccessMask = VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
    .dstAccessMask = VK_ACCESS_SHADER_READ_BIT
};
vkCmdPipelineBarrier(cmdBuf,
    VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
    VK_PIPELINE_STAGE_FRAGMENT_SHADER_BIT,
    0, 1, &barrier, 0, NULL, 0, NULL);`,
      realSoftware: ['MoltenVK', 'ARM Mali GPU Driver', 'Unreal Engine', 'Unity'],
      concurrencyAnalysis: {
        typicalDeployments: {
          small: {
            cores: '32 threads/wg',
            description: '单个GPU工作组',
            barrierBehavior: 'Near-Atomic主导，GPU Tile内部'
          },
          medium: {
            cores: '数百GPU cores',
            description: '中端GPU设备',
            barrierBehavior: '混合模式，Tile内Near，跨Tile Far'
          },
          large: {
            cores: '数千并发线程',
            description: '高端GPU/多GPU',
            barrierBehavior: 'Far-Atomic主导，GPU-CPU同步瓶颈'
          }
        },
        performanceByCores: [
          { cores: 32, latency: '100ns', throughput: '高', bottleneck: '无' },
          { cores: 128, latency: '300ns', throughput: '高', bottleneck: 'Tile同步' },
          { cores: 512, latency: '1us', throughput: '中', bottleneck: 'GPU-CPU互连' },
          { cores: 2048, latency: '5us', throughput: '低', bottleneck: 'Pipeline Barrier' }
        ]
      }
    },
    {
      id: 'blockchain-consensus',
      name: '区块链共识',
      icon: '⛓',
      color: '#ff9500',
      category: '区块链',
      barrierFrequency: 'medium',
      optimalAtomicMode: 'far',
      description: 'Hyperledger Fabric等联盟链的PBFT共识协议中，共识阶段需要多轮投票和屏障同步。Pre-prepare→Prepare→Commit每阶段都需要法定人数(quorum)确认。',
      barrierRole: '确保足够数量的副本节点达成一致后进入下一共识阶段',
      phases: [
        { name: 'Pre-prepare', desc: 'Leader提出提案', barrierType: 'explicit' },
        { name: 'Prepare', desc: '副本投票验证', barrierType: 'explicit' },
        { name: 'Commit', desc: '法定人数确认提交', barrierType: 'explicit' }
      ],
      hardwareAnalysis: {
        nearAtomicImpact: '单个节点验证签名和哈希计算时，交易数据以Unique状态缓存在L1中，密码学操作享受~3-5 cycles的Near-Atomic延迟。但PBFT的3阶段共识需要等待2f+1个节点的投票，本地验证完成后必须进入Far-Atomic的投票收集阶段。',
        farAtomicImpact: 'PBFT共识的每阶段Barrier（Pre-prepare、Prepare、Commit）都需要跨节点投票。投票消息在不同节点间以Shared状态传播，每次L1未命中通过CHI互连触发~20-100+ cycles的Far-Atomic延迟。跨地域部署中，网络延迟（ms级）远超Far-Atomic，但Far-Atomic仍是同数据中心共识的关键开销。',
        cacheTopology: {
          sameCluster: '~0.5-3ms (同数据中心，Far-Atomic为主)',
          crossCluster: '~5-30ms (同城双活，网络+Far-Atomic)',
          crossNuma: '~50-300ms (跨地域共识)'
        }
      },
      armPerformance: {
        graviton2: { cores: 64, barrierLatency: '~8ms (PBFT round)', throughput: '3000 TPS' },
        graviton3: { cores: 64, barrierLatency: '~5ms (PBFT round)', throughput: '5000 TPS' },
        graviton4: { cores: 96, barrierLatency: '~3ms (PBFT round)', throughput: '8000 TPS' }
      },
      codeExample: `// PBFT共识简化伪代码
func PBFT_Round(proposal):
    // Phase 1: Pre-prepare (Leader)
    broadcast(prepare_msg(proposal))

    // Phase 2: Prepare - 等待2f+1个prepare投票
    barrier.wait_quorum(2*f+1, "prepare")

    // Phase 3: Commit - 等待2f+1个commit确认
    barrier.wait_quorum(2*f+1, "commit")

    execute(proposal)`,
      realSoftware: ['Hyperledger Fabric', 'Tendermint', 'PBFT implementations'],
      concurrencyAnalysis: {
        typicalDeployments: {
          small: {
            cores: '4验证节点',
            description: '开发/测试网络',
            barrierBehavior: 'Near-Atomic主导，本地验证'
          },
          medium: {
            cores: '8-16验证节点',
            description: '联盟链生产环境',
            barrierBehavior: '混合模式，本地Near，投票Far'
          },
          large: {
            cores: '数十验证节点',
            description: '大规模联盟链',
            barrierBehavior: 'Far-Atomic主导，PBFT投票累积'
          }
        },
        performanceByCores: [
          { cores: 4, latency: '2ms', throughput: '高', bottleneck: '无' },
          { cores: 8, latency: '5ms', throughput: '中', bottleneck: '投票收集' },
          { cores: 16, latency: '15ms', throughput: '低', bottleneck: '共识轮次' },
          { cores: 32, latency: '40ms', throughput: '极低', bottleneck: '网络+投票延迟' }
        ]
      }
    }
  ],

  // 决策树数据
  decisionTree: {
    root: {
      question: '临界区长度?',
      type: 'decision',
      children: {
        '短 (< 100 cycles)': {
          result: 'spinlock',
          recommendation: '使用自旋锁类 (Simple Spinlock / TTAS)',
          children: {
            '高竞争?': {
              '是': { result: 'mcs-spinlock', recommendation: '使用MCS队列自旋锁' },
              '否': { result: 'ttas', recommendation: '使用TTAS锁' }
            }
          }
        },
        '长 (> 1000 cycles)': {
          next: 'competition'
        },
        '中等': {
          next: 'competition'
        }
      }
    },
    competition: {
      question: '竞争程度?',
      type: 'decision',
      children: {
        '低': {
          result: 'futex',
          recommendation: '使用Futex (通用最佳选择)'
        },
        '高': {
          next: 'fairness'
        },
        '不确定': {
          result: 'futex',
          recommendation: '使用Futex (自适应竞争)'
        }
      }
    },
    fairness: {
      question: '需要公平性保证?',
      type: 'decision',
      children: {
        '是': {
          result: 'ticket-spinlock',
          recommendation: '使用Ticket自旋锁或MCS锁'
        },
        '否': {
          result: 'mutex',
          recommendation: '使用Mutex'
        }
      }
    }
  },

  // 获取所有关键字
  getAllKeywords: function() {
    const keywords = new Set();
    this.locks.forEach(lock => {
      lock.keywords.forEach(k => keywords.add(k.toLowerCase()));
    });
    return Array.from(keywords);
  },

  // 按分类获取锁
  getLocksByCategory: function(categoryId) {
    return this.locks.filter(lock => lock.category === categoryId);
  },

  // 搜索锁
  searchLocks: function(query) {
    const q = query.toLowerCase();
    return this.locks.filter(lock =>
      lock.name.toLowerCase().includes(q) ||
      lock.nameEn.toLowerCase().includes(q) ||
      lock.keywords.some(k => k.toLowerCase().includes(q)) ||
      lock.principle.toLowerCase().includes(q)
    );
  }
};

// 导出
if (typeof module !== 'undefined' && module.exports) {
  module.exports = LockData;
}
