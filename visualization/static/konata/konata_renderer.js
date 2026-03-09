/**
 * Konata Pipeline Renderer
 *
 * A Canvas 2D renderer for pipeline visualization based on the Konata format.
 * Provides detailed stage-by-stage visualization of instruction flow through
 * the CPU pipeline with dependency arrows.
 *
 * Note: STAGE_COLORS is defined in op.js which must be loaded before this file.
 */

/**
 * Default rendering configuration
 */
const DEFAULT_CONFIG = {
    // Layout
    rowHeight: 24,
    labelWidth: 150,
    cycleWidth: 8,
    headerHeight: 30,
    timelineHeight: 40,

    // Colors
    backgroundColor: '#1a1a2e',
    gridColor: '#2a2a4a',
    gridTextColor: '#888888',
    labelBgColor: '#16213e',
    labelTextColor: '#eaeaea',
    highlightColor: '#e94560',

    // Interaction
    minZoom: 0.1,
    maxZoom: 10,
    zoomStep: 1.2,

    // Performance
    maxVisibleOps: 500,
    offscreenBuffer: true
};

/**
 * KonataRenderer - Main rendering class for pipeline visualization
 */
class KonataRenderer {
    constructor(container, config = {}) {
        this.container = container;
        this.config = { ...DEFAULT_CONFIG, ...config };

        // Create canvas
        this.canvas = document.createElement('canvas');
        this.canvas.style.width = '100%';
        this.canvas.style.height = '100%';
        this.canvas.style.cursor = 'grab';
        container.appendChild(this.canvas);

        this.ctx = this.canvas.getContext('2d');

        // State
        this.ops = [];
        this.opsMap = new Map();

        // View state
        this.scrollX = 0;
        this.scrollY = 0;
        this.zoom = 1;
        this.cycleOffset = 0;

        // Interaction state
        this.isDragging = false;
        this.dragStartX = 0;
        this.dragStartY = 0;
        this.lastScrollX = 0;
        this.lastScrollY = 0;

        // Selection
        this.selectedOp = null;
        this.hoveredOp = null;
        this.searchResults = [];
        this.searchIndex = -1;

        // Layout cache
        this.layoutCache = null;
        this.needsLayout = true;

        // Bind event handlers
        this.bindEvents();

        // Initial resize
        this.resize();
    }

    /**
     * Bind mouse and keyboard events
     */
    bindEvents() {
        // Mouse events
        this.canvas.addEventListener('mousedown', this.onMouseDown.bind(this));
        this.canvas.addEventListener('mousemove', this.onMouseMove.bind(this));
        this.canvas.addEventListener('mouseup', this.onMouseUp.bind(this));
        this.canvas.addEventListener('mouseleave', this.onMouseUp.bind(this));
        this.canvas.addEventListener('wheel', this.onWheel.bind(this), { passive: false });
        this.canvas.addEventListener('dblclick', this.onDoubleClick.bind(this));

        // Touch events
        this.canvas.addEventListener('touchstart', this.onTouchStart.bind(this));
        this.canvas.addEventListener('touchmove', this.onTouchMove.bind(this));
        this.canvas.addEventListener('touchend', this.onTouchEnd.bind(this));

        // Keyboard events
        this.canvas.setAttribute('tabindex', '0');
        this.canvas.addEventListener('keydown', this.onKeyDown.bind(this));

        // Resize
        window.addEventListener('resize', () => this.resize());
    }

    /**
     * Handle canvas resize
     */
    resize() {
        const rect = this.container.getBoundingClientRect();
        const dpr = window.devicePixelRatio || 1;

        console.log('KonataRenderer resize:', rect.width, 'x', rect.height);

        // Handle case where container is not visible yet
        if (rect.width <= 0 || rect.height <= 0) {
            console.warn('KonataRenderer: Container has no dimensions, retrying...');
            // Try again after a short delay
            setTimeout(() => this.resize(), 100);
            return;
        }

        this.canvas.width = rect.width * dpr;
        this.canvas.height = rect.height * dpr;
        this.ctx.setTransform(1, 0, 0, 1, 0, 0); // Reset transform
        this.ctx.scale(dpr, dpr);

        this.width = rect.width;
        this.height = rect.height;

        this.needsLayout = true;
        this.render();
    }

    /**
     * Set operations data
     */
    setOps(ops) {
        console.log('KonataRenderer setOps:', ops ? ops.length : 0, 'ops');

        // Convert raw data to internal format if Op class is available
        if (typeof Op !== 'undefined') {
            this.ops = ops.map(op => op instanceof Op ? op : new Op(op));
        } else {
            // Use raw data directly with adapter methods
            this.ops = ops.map(op => this.createOpAdapter(op));
        }

        this.opsMap.clear();
        this.ops.forEach(op => this.opsMap.set(op.id, op));

        this.needsLayout = true;
        this.render();
    }

    /**
     * Create an adapter object that mimics Op class from raw data
     */
    createOpAdapter(data) {
        const op = {
            id: data.id,
            gid: data.gid,
            rid: data.rid,
            fetchedCycle: data.fetched_cycle || data.fetchedCycle || 0,
            retiredCycle: data.retired_cycle || data.retiredCycle,
            labelName: data.label_name || data.labelName || '',
            pc: data.pc,
            srcRegs: data.src_regs || data.srcRegs || [],
            dstRegs: data.dst_regs || data.dstRegs || [],
            isMemory: data.is_memory || data.isMemory || false,
            memAddr: data.mem_addr || data.memAddr,
            lanes: new Map(),
            prods: (data.prods || []).map(d => ({
                producerId: d.producer_id !== undefined ? d.producer_id : d.producerId,
                depType: d.dep_type !== undefined ? d.dep_type : d.depType,
                color: (d.dep_type !== undefined ? d.dep_type : d.depType) === 'memory' ? '#0066ff' : '#ff6600'
            })),
            x: 0, y: 0, width: 0, height: 0, visible: true, highlighted: false
        };

        // Parse lanes
        if (data.lanes) {
            for (const [laneName, laneData] of Object.entries(data.lanes)) {
                const stages = (laneData.stages || []).map(s => {
                    const stageColor = (STAGE_COLORS && STAGE_COLORS[s.name]) || { h: 0, s: 0, l: 50, name: s.name };
                    return {
                        name: s.name,
                        startCycle: s.start_cycle || s.startCycle || 0,
                        endCycle: s.end_cycle || s.endCycle || 0,
                        color: stageColor,
                        get cssColor() {
                            return `hsl(${this.color.h}, ${this.color.s}%, ${this.color.l}%)`;
                        },
                        cssColorTransparent(alpha = 0.3) {
                            return `hsla(${this.color.h}, ${this.color.s}%, ${this.color.l}%, ${alpha})`;
                        }
                    };
                });
                op.lanes.set(laneName, { name: laneName, stages });
            }
        }

        // Add helper methods as getters (so they can be accessed as properties)
        Object.defineProperty(op, 'earliestCycle', {
            get: function() {
                let min = this.fetchedCycle;
                for (const lane of this.lanes.values()) {
                    for (const stage of lane.stages) {
                        min = Math.min(min, stage.startCycle);
                    }
                }
                return min;
            },
            enumerable: true
        });

        Object.defineProperty(op, 'latestCycle', {
            get: function() {
                let max = this.retiredCycle || 0;
                for (const lane of this.lanes.values()) {
                    for (const stage of lane.stages) {
                        max = Math.max(max, stage.endCycle);
                    }
                }
                return max;
            },
            enumerable: true
        });

        op.formatPC = function() {
            return '0x' + this.pc.toString(16).padStart(8, '0');
        };

        return op;
    }

    /**
     * Update with new snapshot data
     */
    updateSnapshot(snapshot) {
        console.log('KonataRenderer updateSnapshot:', snapshot ? (snapshot.ops ? snapshot.ops.length : 'no ops') : 'null');
        if (snapshot && snapshot.ops) {
            this.setOps(snapshot.ops);
        }
    }

    /**
     * Calculate layout for all operations
     */
    calculateLayout() {
        if (!this.needsLayout && this.layoutCache) {
            return this.layoutCache;
        }

        const layout = {
            minY: 0,
            maxY: 0,
            minCycle: Infinity,
            maxCycle: 0,
            rowHeight: this.config.rowHeight,
            labelWidth: this.config.labelWidth,
            cycleWidth: this.config.cycleWidth * this.zoom
        };

        // Calculate bounds
        for (const op of this.ops) {
            layout.minCycle = Math.min(layout.minCycle, op.earliestCycle);
            layout.maxCycle = Math.max(layout.maxCycle, op.latestCycle);
        }

        if (layout.minCycle === Infinity) {
            layout.minCycle = 0;
        }

        // Cycle to X coordinate conversion
        layout.cycleToX = (cycle) => {
            return this.config.labelWidth +
                (cycle - layout.minCycle - this.cycleOffset) * layout.cycleWidth -
                this.scrollX;
        };

        // X coordinate to cycle conversion
        layout.xToCycle = (x) => {
            return layout.minCycle + this.cycleOffset +
                (x + this.scrollX - this.config.labelWidth) / layout.cycleWidth;
        };

        // Op index to Y coordinate conversion
        layout.opToY = (index) => {
            return this.config.headerHeight + this.config.timelineHeight +
                index * layout.rowHeight - this.scrollY;
        };

        // Y coordinate to op index conversion
        layout.yToOpIndex = (y) => {
            return Math.floor((y + this.scrollY - this.config.headerHeight - this.config.timelineHeight) / layout.rowHeight);
        };

        layout.maxY = this.ops.length * layout.rowHeight;

        this.layoutCache = layout;
        this.needsLayout = false;

        return layout;
    }

    /**
     * Main render function
     */
    render() {
        // Skip if canvas has no dimensions
        if (this.width <= 0 || this.height <= 0) {
            console.log('KonataRenderer render: skipping, invalid dimensions', this.width, 'x', this.height);
            return;
        }

        const ctx = this.ctx;
        const layout = this.calculateLayout();

        console.log('KonataRenderer render: width=', this.width, 'height=', this.height, 'ops=', this.ops.length);

        // Clear canvas
        ctx.fillStyle = this.config.backgroundColor;
        ctx.fillRect(0, 0, this.width, this.height);

        if (this.ops.length === 0) {
            this.renderEmptyState(ctx);
            return;
        }

        // Render layers
        this.renderGrid(ctx, layout);
        this.renderTimeline(ctx, layout);
        this.renderLabels(ctx, layout);
        this.renderOps(ctx, layout);
        this.renderDependencies(ctx, layout);

        // Render selection highlight
        if (this.selectedOp) {
            this.renderSelection(ctx, layout, this.selectedOp);
        }
    }

    /**
     * Render empty state message
     */
    renderEmptyState(ctx) {
        ctx.fillStyle = this.config.labelTextColor;
        ctx.font = '14px sans-serif';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText('No pipeline data available', this.width / 2, this.height / 2);
    }

    /**
     * Render background grid
     */
    renderGrid(ctx, layout) {
        ctx.strokeStyle = this.config.gridColor;
        ctx.lineWidth = 1;

        // Calculate visible cycle range
        const startCycle = Math.floor(layout.xToCycle(this.config.labelWidth));
        const endCycle = Math.ceil(layout.xToCycle(this.width));

        // Draw vertical grid lines (every 10 cycles at default zoom)
        const gridStep = Math.max(1, Math.floor(10 / this.zoom));
        const firstGridCycle = Math.ceil(startCycle / gridStep) * gridStep;

        ctx.beginPath();
        for (let cycle = firstGridCycle; cycle <= endCycle; cycle += gridStep) {
            const x = layout.cycleToX(cycle);
            if (x >= this.config.labelWidth && x <= this.width) {
                ctx.moveTo(x, 0);
                ctx.lineTo(x, this.height);
            }
        }
        ctx.stroke();

        // Draw horizontal grid lines
        const startIdx = Math.max(0, layout.yToOpIndex(this.config.headerHeight + this.config.timelineHeight));
        const endIdx = Math.min(this.ops.length - 1, layout.yToOpIndex(this.height));

        ctx.beginPath();
        for (let i = startIdx; i <= endIdx; i++) {
            const y = layout.opToY(i);
            ctx.moveTo(this.config.labelWidth, y);
            ctx.lineTo(this.width, y);
        }
        ctx.stroke();
    }

    /**
     * Render timeline with cycle numbers
     */
    renderTimeline(ctx, layout) {
        const timelineY = this.config.headerHeight;
        const timelineHeight = this.config.timelineHeight;

        // Timeline background
        ctx.fillStyle = this.config.labelBgColor;
        ctx.fillRect(0, timelineY, this.width, timelineHeight);

        // Calculate visible cycle range
        const startCycle = Math.floor(layout.xToCycle(this.config.labelWidth));
        const endCycle = Math.ceil(layout.xToCycle(this.width));

        // Draw cycle labels
        ctx.fillStyle = this.config.gridTextColor;
        ctx.font = '11px monospace';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';

        const labelStep = this.getLabelStep();
        const firstLabelCycle = Math.ceil(startCycle / labelStep) * labelStep;

        for (let cycle = firstLabelCycle; cycle <= endCycle; cycle += labelStep) {
            const x = layout.cycleToX(cycle);
            if (x >= this.config.labelWidth && x <= this.width) {
                ctx.fillText(cycle.toString(), x, timelineY + timelineHeight / 2);

                // Tick mark
                ctx.strokeStyle = this.config.gridColor;
                ctx.beginPath();
                ctx.moveTo(x, timelineY + timelineHeight - 5);
                ctx.lineTo(x, timelineY + timelineHeight);
                ctx.stroke();
            }
        }

        // Draw separator line
        ctx.strokeStyle = this.config.gridColor;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(0, timelineY + timelineHeight);
        ctx.lineTo(this.width, timelineY + timelineHeight);
        ctx.stroke();
        ctx.lineWidth = 1;
    }

    /**
     * Get appropriate label step based on zoom
     */
    getLabelStep() {
        if (this.zoom >= 2) return 1;
        if (this.zoom >= 1) return 5;
        if (this.zoom >= 0.5) return 10;
        if (this.zoom >= 0.25) return 20;
        return 50;
    }

    /**
     * Render instruction labels
     */
    renderLabels(ctx, layout) {
        const labelWidth = this.config.labelWidth;
        const labelX = 0;

        // Label header
        ctx.fillStyle = this.config.labelBgColor;
        ctx.fillRect(0, 0, labelWidth, this.config.headerHeight);

        ctx.fillStyle = this.config.labelTextColor;
        ctx.font = 'bold 12px sans-serif';
        ctx.textAlign = 'left';
        ctx.textBaseline = 'middle';
        ctx.fillText('Instruction', 10, this.config.headerHeight / 2);

        // Calculate visible range
        const startIdx = Math.max(0, layout.yToOpIndex(this.config.headerHeight + this.config.timelineHeight));
        const endIdx = Math.min(this.ops.length - 1, layout.yToOpIndex(this.height));

        // Draw labels for visible ops
        for (let i = startIdx; i <= endIdx; i++) {
            const op = this.ops[i];
            const y = layout.opToY(i);

            // Background
            const isSelected = this.selectedOp && this.selectedOp.id === op.id;
            const isHovered = this.hoveredOp && this.hoveredOp.id === op.id;
            const isSearchResult = this.searchResults.includes(op.id);

            ctx.fillStyle = isSelected ? this.config.highlightColor :
                isHovered ? '#1f3a5f' :
                    isSearchResult ? '#3a1f5f' :
                        this.config.labelBgColor;
            ctx.fillRect(labelX, y, labelWidth, layout.rowHeight);

            // Border
            ctx.strokeStyle = this.config.gridColor;
            ctx.strokeRect(labelX, y, labelWidth, layout.rowHeight);

            // Text
            ctx.fillStyle = isSelected ? '#ffffff' : this.config.labelTextColor;
            ctx.font = '11px monospace';
            ctx.textAlign = 'left';
            ctx.textBaseline = 'middle';

            // Format label: ID + PC + opcode
            const label = `[${op.id}] ${op.formatPC()} ${op.labelName.substring(0, 12)}`;
            ctx.fillText(label, 5, y + layout.rowHeight / 2, labelWidth - 10);
        }

        // Vertical separator
        ctx.strokeStyle = this.config.gridColor;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(labelWidth, 0);
        ctx.lineTo(labelWidth, this.height);
        ctx.stroke();
        ctx.lineWidth = 1;
    }

    /**
     * Render pipeline stages for all visible operations
     */
    renderOps(ctx, layout) {
        const startIdx = Math.max(0, layout.yToOpIndex(this.config.headerHeight + this.config.timelineHeight));
        const endIdx = Math.min(this.ops.length - 1, layout.yToOpIndex(this.height));

        for (let i = startIdx; i <= endIdx; i++) {
            const op = this.ops[i];
            const y = layout.opToY(i);
            const isSelected = this.selectedOp && this.selectedOp.id === op.id;
            const isHovered = this.hoveredOp && this.hoveredOp.id === op.id;

            // Render stages
            if (typeof renderOpStages === 'function') {
                renderOpStages(ctx, op, {
                    y,
                    cycleToX: layout.cycleToX,
                    cycleWidth: layout.cycleWidth,
                    rowHeight: layout.rowHeight,
                    showLabels: layout.cycleWidth > 15,
                    highlighted: isSelected || isHovered,
                    alpha: 1.0
                });
            } else {
                // Fallback rendering
                this.renderOpStagesFallback(ctx, op, layout, y);
            }
        }
    }

    /**
     * Fallback stage rendering if stage.js not loaded
     */
    renderOpStagesFallback(ctx, op, layout, y) {
        const stageHeight = layout.rowHeight - 4;
        const stageY = y + 2;

        // Color mapping for stages (matching backend stage names)
        const stageColors = {
            'F': { fill: 'hsla(200, 70%, 60%, 0.7)', stroke: 'hsl(200, 70%, 60%)' },      // Fetch - blue
            'Dc': { fill: 'hsla(180, 60%, 55%, 0.7)', stroke: 'hsl(180, 60%, 55%)' },    // Decode - cyan
            'Rn': { fill: 'hsla(160, 50%, 50%, 0.7)', stroke: 'hsl(160, 50%, 50%)' },    // Rename - teal
            'Ds': { fill: 'hsla(140, 60%, 55%, 0.7)', stroke: 'hsl(140, 60%, 55%)' },    // Dispatch - green
            'Is': { fill: 'hsla(120, 70%, 45%, 0.7)', stroke: 'hsl(120, 70%, 45%)' },    // Issue - yellow-green
            'Ex': { fill: 'hsla(60, 80%, 55%, 0.7)', stroke: 'hsl(60, 80%, 55%)' },      // Execute - yellow
            'Me': { fill: 'hsla(30, 80%, 55%, 0.7)', stroke: 'hsl(30, 80%, 55%)' },      // Memory - orange
            'Cm': { fill: 'hsla(280, 60%, 55%, 0.7)', stroke: 'hsl(280, 60%, 55%)' },    // Complete - purple
            'Rt': { fill: 'hsla(320, 50%, 50%, 0.7)', stroke: 'hsl(320, 50%, 50%)' }       // Retire - pink
        };

        for (const lane of op.lanes.values()) {
            for (const stage of lane.stages) {
                const startX = layout.cycleToX(stage.startCycle);
                const endX = layout.cycleToX(stage.endCycle);
                const width = Math.max(endX - startX, layout.cycleWidth);

                // Get color for this stage, use stage.cssColor if available, otherwise use mapping
                let fillColor, strokeColor;
                if (stage.cssColor && stage.cssColorTransparent) {
                    fillColor = stage.cssColorTransparent(0.7);
                    strokeColor = stage.cssColor;
                } else {
                    const colors = stageColors[stage.name] || { fill: 'hsla(0, 0%, 50%, 0.7)', stroke: 'hsla(0, 0%, 50%)' };
                    fillColor = colors.fill;
                    strokeColor = colors.stroke;
                }

                ctx.fillStyle = fillColor;
                ctx.strokeStyle = strokeColor;

                // Use custom roundRect for compatibility
                this.drawRoundRect(ctx, startX, stageY, width, stageHeight, 3);
                ctx.fill();
                ctx.stroke();
            }
        }
    }

    /**
     * Draw a rounded rectangle (compatible with older browsers)
     */
    drawRoundRect(ctx, x, y, width, height, radius) {
        radius = Math.max(0, Math.min(radius, width / 2, height / 2));
        ctx.beginPath();
        ctx.moveTo(x + radius, y);
        ctx.lineTo(x + width - radius, y);
        ctx.quadraticCurveTo(x + width, y, x + width, y + radius);
        ctx.lineTo(x + width, y + height - radius);
        ctx.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
        ctx.lineTo(x + radius, y + height);
        ctx.quadraticCurveTo(x, y + height, x, y + height - radius);
        ctx.lineTo(x, y + radius);
        ctx.quadraticCurveTo(x, y, x + radius, y);
        ctx.closePath();
    }

    /**
     * Render dependency arrows
     * Arrows go from producer's complete cycle to consumer's issue cycle (when execution starts)
     * This correctly represents the dependency: consumer must wait for producer to complete
     */
    renderDependencies(ctx, layout) {
        const startIdx = Math.max(0, layout.yToOpIndex(this.config.headerHeight + this.config.timelineHeight) - 5);
        const endIdx = Math.min(this.ops.length - 1, layout.yToOpIndex(this.height) + 5);

        for (let i = startIdx; i <= endIdx; i++) {
            const op = this.ops[i];
            const consumerY = layout.opToY(i) + layout.rowHeight / 2;

            for (const dep of op.prods) {
                const producer = this.opsMap.get(dep.producerId);
                if (!producer) continue;

                const producerIndex = this.ops.indexOf(producer);
                if (producerIndex < startIdx - 5 || producerIndex > endIdx + 5) continue;

                const producerY = layout.opToY(producerIndex) + layout.rowHeight / 2;

                // Get end points for dependency arrow:
                // - Start: producer's complete cycle (when result is available)
                // - End: consumer's issue cycle (when execution starts)
                const producerCompleteCycle = this.getCompleteCycle(producer);
                const consumerIssueCycle = this.getIssueCycle(op);

                const startX = layout.cycleToX(producerCompleteCycle);
                const endX = layout.cycleToX(consumerIssueCycle);

                // Only draw if visible
                if (endX < this.config.labelWidth || startX > this.width) continue;

                // Draw arrow
                this.drawDependencyArrow(ctx, startX, producerY, endX, consumerY, dep.color);
            }
        }
    }

    /**
     * Get the complete cycle for an operation (when result is available)
     * Returns the END cycle of the Execute/Memory stage, which is when the result is ready.
     * Note: We use Execute/Memory END, not Complete END, because the result is available
     * as soon as execution finishes, not after the Complete stage.
     */
    getCompleteCycle(op) {
        // Try to get from stages first (most accurate)
        const mainLane = op.lanes.get('main');
        if (mainLane) {
            // Result is available at Execute/Memory END (not Complete END)
            const execStage = mainLane.stages.find(s => s.name === 'Ex');
            if (execStage) {
                return execStage.endCycle;
            }
            const memStage = mainLane.stages.find(s => s.name === 'Me');
            if (memStage) {
                return memStage.endCycle;
            }
            // Fallback to complete stage START (not END)
            const completeStage = mainLane.stages.find(s => s.name === 'Cm');
            if (completeStage) {
                return completeStage.startCycle;
            }
        }
        // Fallback to retiredCycle or call latestCycle function
        if (op.retiredCycle) return op.retiredCycle;
        if (typeof op.latestCycle === 'function') return op.latestCycle();
        return 0;
    }

    /**
     * Get the issue cycle for an operation (when execution starts)
     */
    getIssueCycle(op) {
        // Try to get from stages first (most accurate)
        const mainLane = op.lanes.get('main');
        if (mainLane) {
            // Find issue stage end (Is) - this is when execution starts
            const issueStage = mainLane.stages.find(s => s.name === 'Is');
            if (issueStage) {
                return issueStage.endCycle;
            }
            // Fallback to execute start
            const execStage = mainLane.stages.find(s => s.name === 'Ex');
            if (execStage) {
                return execStage.startCycle;
            }
            const memStage = mainLane.stages.find(s => s.name === 'Me');
            if (memStage) {
                return memStage.startCycle;
            }
        }
        // Fallback to fetchedCycle
        return op.fetchedCycle || 0;
    }

    /**
     * Draw a dependency arrow
     */
    drawDependencyArrow(ctx, x1, y1, x2, y2, color) {
        ctx.save();

        ctx.strokeStyle = color;
        ctx.fillStyle = color;
        ctx.lineWidth = 1.5;
        ctx.globalAlpha = 0.6;

        // Draw line
        ctx.beginPath();
        ctx.moveTo(x1, y1);

        // Bezier curve for smoother appearance
        const midX = (x1 + x2) / 2;
        ctx.bezierCurveTo(midX, y1, midX, y2, x2, y2);
        ctx.stroke();

        // Draw arrowhead
        const arrowSize = 6;
        const angle = Math.atan2(y2 - y1, x2 - midX);

        ctx.beginPath();
        ctx.moveTo(x2, y2);
        ctx.lineTo(
            x2 - arrowSize * Math.cos(angle - Math.PI / 6),
            y2 - arrowSize * Math.sin(angle - Math.PI / 6)
        );
        ctx.lineTo(
            x2 - arrowSize * Math.cos(angle + Math.PI / 6),
            y2 - arrowSize * Math.sin(angle + Math.PI / 6)
        );
        ctx.closePath();
        ctx.fill();

        ctx.restore();
    }

    /**
     * Render selection highlight
     */
    renderSelection(ctx, layout, op) {
        const index = this.ops.indexOf(op);
        if (index < 0) return;

        const y = layout.opToY(index);

        ctx.save();
        ctx.strokeStyle = this.config.highlightColor;
        ctx.lineWidth = 2;
        ctx.setLineDash([5, 3]);

        ctx.strokeRect(
            this.config.labelWidth,
            y,
            this.width - this.config.labelWidth,
            layout.rowHeight
        );

        ctx.restore();
    }

    /**
     * Mouse event handlers
     */
    onMouseDown(e) {
        this.isDragging = true;
        this.dragStartX = e.clientX;
        this.dragStartY = e.clientY;
        this.lastScrollX = this.scrollX;
        this.lastScrollY = this.scrollY;
        this.canvas.style.cursor = 'grabbing';
    }

    onMouseMove(e) {
        if (this.isDragging) {
            const dx = e.clientX - this.dragStartX;
            const dy = e.clientY - this.dragStartY;

            this.scrollX = this.lastScrollX - dx;
            this.scrollY = Math.max(0, this.lastScrollY - dy);

            this.render();
        } else {
            // Update hovered op
            const rect = this.canvas.getBoundingClientRect();
            const x = e.clientX - rect.left;
            const y = e.clientY - rect.top;

            const layout = this.calculateLayout();
            const opIndex = layout.yToOpIndex(y);

            const prevHovered = this.hoveredOp;
            if (opIndex >= 0 && opIndex < this.ops.length) {
                this.hoveredOp = this.ops[opIndex];
            } else {
                this.hoveredOp = null;
            }

            if (prevHovered !== this.hoveredOp) {
                this.render();
            }
        }
    }

    onMouseUp(e) {
        this.isDragging = false;
        this.canvas.style.cursor = 'grab';
    }

    onWheel(e) {
        e.preventDefault();

        const rect = this.canvas.getBoundingClientRect();
        const mouseX = e.clientX - rect.left;

        if (e.ctrlKey || e.metaKey) {
            // Zoom
            const delta = e.deltaY > 0 ? 1 / this.config.zoomStep : this.config.zoomStep;
            const newZoom = Math.max(this.config.minZoom, Math.min(this.config.maxZoom, this.zoom * delta));

            // Zoom towards mouse position
            const layout = this.calculateLayout();
            const cycleAtMouse = layout.xToCycle(mouseX);

            this.zoom = newZoom;
            this.needsLayout = true;

            const newLayout = this.calculateLayout();
            const newMouseX = newLayout.cycleToX(cycleAtMouse);
            this.scrollX += newMouseX - mouseX;
        } else {
            // Scroll
            this.scrollX += e.deltaX;
            this.scrollY = Math.max(0, this.scrollY + e.deltaY);
        }

        this.render();
    }

    onDoubleClick(e) {
        const rect = this.canvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;

        const layout = this.calculateLayout();
        const opIndex = layout.yToOpIndex(y);

        if (opIndex >= 0 && opIndex < this.ops.length) {
            this.selectedOp = this.ops[opIndex];
            this.render();

            // Emit selection event
            this.emit('select', this.selectedOp);
        }
    }

    /**
     * Touch event handlers
     */
    onTouchStart(e) {
        if (e.touches.length === 1) {
            const touch = e.touches[0];
            this.isDragging = true;
            this.dragStartX = touch.clientX;
            this.dragStartY = touch.clientY;
            this.lastScrollX = this.scrollX;
            this.lastScrollY = this.scrollY;
        }
    }

    onTouchMove(e) {
        if (e.touches.length === 1 && this.isDragging) {
            e.preventDefault();
            const touch = e.touches[0];
            const dx = touch.clientX - this.dragStartX;
            const dy = touch.clientY - this.dragStartY;

            this.scrollX = this.lastScrollX - dx;
            this.scrollY = Math.max(0, this.lastScrollY - dy);

            this.render();
        }
    }

    onTouchEnd(e) {
        this.isDragging = false;
    }

    /**
     * Keyboard event handler
     */
    onKeyDown(e) {
        switch (e.key) {
            case 'ArrowLeft':
                this.scrollX -= 50;
                this.render();
                break;
            case 'ArrowRight':
                this.scrollX += 50;
                this.render();
                break;
            case 'ArrowUp':
                this.scrollY = Math.max(0, this.scrollY - 50);
                this.render();
                break;
            case 'ArrowDown':
                this.scrollY += 50;
                this.render();
                break;
            case '+':
            case '=':
                this.zoomIn();
                break;
            case '-':
                this.zoomOut();
                break;
            case 'Escape':
                this.selectedOp = null;
                this.searchResults = [];
                this.render();
                break;
            case 'Enter':
                if (this.searchResults.length > 0) {
                    this.nextSearchResult();
                }
                break;
        }
    }

    /**
     * Zoom controls
     */
    zoomIn() {
        this.zoom = Math.min(this.config.maxZoom, this.zoom * this.config.zoomStep);
        this.needsLayout = true;
        this.render();
    }

    zoomOut() {
        this.zoom = Math.max(this.config.minZoom, this.zoom / this.config.zoomStep);
        this.needsLayout = true;
        this.render();
    }

    resetView() {
        this.scrollX = 0;
        this.scrollY = 0;
        this.zoom = 1;
        this.cycleOffset = 0;
        this.needsLayout = true;
        this.render();
    }

    /**
     * Search functionality
     */
    search(query) {
        this.searchResults = [];
        this.searchIndex = -1;

        if (!query) return;

        const lowerQuery = query.toLowerCase();

        for (const op of this.ops) {
            if (op.labelName.toLowerCase().includes(lowerQuery) ||
                op.formatPC().toLowerCase().includes(lowerQuery) ||
                op.id.toString().includes(query)) {
                this.searchResults.push(op.id);
            }
        }

        if (this.searchResults.length > 0) {
            this.nextSearchResult();
        }

        this.render();
        return this.searchResults.length;
    }

    nextSearchResult() {
        if (this.searchResults.length === 0) return;

        this.searchIndex = (this.searchIndex + 1) % this.searchResults.length;
        const opId = this.searchResults[this.searchIndex];
        const op = this.opsMap.get(opId);

        if (op) {
            this.selectedOp = op;
            this.scrollToOp(op);
            this.emit('select', op);
        }

        this.render();
    }

    prevSearchResult() {
        if (this.searchResults.length === 0) return;

        this.searchIndex = (this.searchIndex - 1 + this.searchResults.length) % this.searchResults.length;
        const opId = this.searchResults[this.searchIndex];
        const op = this.opsMap.get(opId);

        if (op) {
            this.selectedOp = op;
            this.scrollToOp(op);
            this.emit('select', op);
        }

        this.render();
    }

    scrollToOp(op) {
        const layout = this.calculateLayout();
        const index = this.ops.indexOf(op);

        if (index >= 0) {
            // Center vertically
            const opY = index * layout.rowHeight;
            this.scrollY = Math.max(0, opY - this.height / 2 + layout.rowHeight);

            // Ensure start cycle is visible
            const startX = layout.cycleToX(op.fetchedCycle);
            if (startX < this.config.labelWidth) {
                this.scrollX = 0;
                this.cycleOffset = op.fetchedCycle;
                this.needsLayout = true;
            }
        }
    }

    /**
     * Event emission
     */
    emit(event, data) {
        const customEvent = new CustomEvent(`konata:${event}`, { detail: data });
        this.container.dispatchEvent(customEvent);
    }

    on(event, handler) {
        this.container.addEventListener(`konata:${event}`, handler);
    }

    off(event, handler) {
        this.container.removeEventListener(`konata:${event}`, handler);
    }

    /**
     * Export data
     */
    exportData() {
        return {
            ops: this.ops.map(op => ({
                id: op.id,
                gid: op.gid,
                pc: op.pc,
                label_name: op.labelName,
                fetched_cycle: op.fetchedCycle,
                retired_cycle: op.retiredCycle,
                lanes: Object.fromEntries(
                    Array.from(op.lanes.entries()).map(([name, lane]) => [
                        name,
                        { stages: lane.stages.map(s => ({ name: s.name, start_cycle: s.startCycle, end_cycle: s.endCycle })) }
                    ])
                ),
                prods: op.prods.map(d => ({ producer_id: d.producerId, dep_type: d.depType }))
            }))
        };
    }

    /**
     * Cleanup
     */
    destroy() {
        window.removeEventListener('resize', this.resize);
        this.canvas.remove();
    }
}

// Export for module usage
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { KonataRenderer, DEFAULT_CONFIG };
}
