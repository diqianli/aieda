/**
 * ARM Lock Mechanisms Data Definitions
 * Contains all lock types, categories, performance metrics, and pseudocode
 */

const LockData = {
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
      warnings: ['不要在持有自旋锁时调用可能睡眠的函数', '临界区应尽可能短（建议 < 100 cycles）', '避免在自旋锁临界区内调用其他可能获取锁的函数（死锁风险）']
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
      warnings: ['仍然不能保证公平性', '高竞争时性能会下降', '不适合极长临界区']
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
      warnings: ['票号变量可能溢出，需要使用足够大的整数类型', '所有等待者仍轮询同一变量，存在缓存行颠簸', '不适合极高竞争场景（考虑MCS）']
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
      warnings: ['必须确保节点内存在锁持有期间保持有效', '每CPU需要预分配节点', '低竞争时额外开销可能不值得']
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
      metrics: { latency: 3, throughput: 4, fairness: 5, power: 4, scalability: 4 }
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
      warnings: ['不能在中断上下文使用', '临界区过长会影响系统响应性', '注意优先级反转问题（使用PI mutex）']
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
      warnings: ['是Linux特有接口，移植性考虑', '状态值管理复杂，容易出错', '直接使用futex(2)需要小心处理边界情况']
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
      metrics: { latency: 5, throughput: 3, fairness: 1, power: 2, scalability: 3 }
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
      metrics: { latency: 3, throughput: 1, fairness: 3, power: 3, scalability: 1 }
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
      metrics: { latency: 4, throughput: 5, fairness: 3, power: 4, scalability: 5 }
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
      metrics: { latency: 3, throughput: 4, fairness: 4, power: 3, scalability: 4 }
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
      metrics: { latency: 2, throughput: 3, fairness: 5, power: 3, scalability: 3 }
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
      metrics: { latency: 4, throughput: 5, fairness: 3, power: 5, scalability: 5 }
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
      metrics: { latency: 3, throughput: 3, fairness: 3, power: 3, scalability: 3 }
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
