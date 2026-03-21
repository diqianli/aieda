// ARM CPU Emulator Static Visualization Client
// Loads Konata JSON data from file instead of WebSocket

class StaticVisualization {
    constructor() {
        this.pipelineView = null;
        this.data = null;

        // DOM elements
        this.elements = {
            totalCycles: document.getElementById('totalCycles'),
            totalInstructions: document.getElementById('totalInstructions'),
            opsCount: document.getElementById('opsCount'),
            fileInput: document.getElementById('fileInput'),
            reloadBtn: document.getElementById('reloadBtn'),
            statusBar: document.getElementById('statusBar'),
            pipelineViewContainer: document.getElementById('pipelineViewContainer')
        };

        this.init();
    }

    init() {
        // Initialize event listeners
        this.elements.fileInput.addEventListener('change', (e) => this.handleFileSelect(e));
        this.elements.reloadBtn.addEventListener('click', () => this.loadDefaultFile());

        // Initialize pipeline view
        this.initPipelineView();

        // Try to load default file
        this.loadDefaultFile();
    }

    initPipelineView() {
        if (typeof PipelineView === 'undefined') {
            console.error('PipelineView not loaded');
            this.elements.pipelineViewContainer.innerHTML =
                '<div style="color: #e94560; padding: 20px; text-align: center;">PipelineView not loaded. Check console for errors.</div>';
            return;
        }

        this.pipelineView = new PipelineView('pipelineViewContainer', {
            autoConnect: false  // Don't connect to WebSocket
        });

        console.log('PipelineView initialized');
    }

    async loadDefaultFile() {
        this.updateStatus('Loading konata_data.json...');

        try {
            const response = await fetch('/konata_data.json');
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            const data = await response.json();
            this.handleData(data);
            this.updateStatus('Loaded konata_data.json successfully');
        } catch (error) {
            console.warn('Could not load default file:', error.message);
            this.updateStatus('Could not load konata_data.json. Please select a file manually.');
        }
    }

    handleFileSelect(event) {
        const file = event.target.files[0];
        if (!file) return;

        this.updateStatus(`Loading ${file.name}...`);

        const reader = new FileReader();
        reader.onload = (e) => {
            try {
                const data = JSON.parse(e.target.result);
                this.handleData(data);
                this.updateStatus(`Loaded ${file.name} successfully`);
            } catch (error) {
                console.error('Failed to parse JSON:', error);
                this.updateStatus(`Error: Failed to parse JSON - ${error.message}`);
            }
        };
        reader.onerror = () => {
            this.updateStatus('Error reading file');
        };
        reader.readAsText(file);
    }

    handleData(data) {
        console.log('Received data:', data);
        this.data = data;

        // Update metrics
        this.elements.totalCycles.textContent = data.total_cycles?.toLocaleString() || '-';
        this.elements.totalInstructions.textContent = data.total_instructions?.toLocaleString() || '-';
        this.elements.opsCount.textContent = data.ops_count?.toLocaleString() || data.ops?.length?.toLocaleString() || '-';

        // Convert to Konata snapshot format if needed
        const snapshot = this.normalizeData(data);

        // Update pipeline view
        if (this.pipelineView && snapshot.ops && snapshot.ops.length > 0) {
            console.log('Updating pipeline view with', snapshot.ops.length, 'ops');
            this.pipelineView.updateSnapshot(snapshot);
        } else {
            console.warn('Pipeline view not ready or no ops in data');
        }
    }

    normalizeData(data) {
        // If data already has ops array, it's in the right format
        if (data.ops && Array.isArray(data.ops)) {
            return {
                cycle: data.total_cycles || 0,
                committed_count: data.total_instructions || 0,
                ops: data.ops,
                metadata: data.metadata || {}
            };
        }

        // If data is an array of snapshots, merge them
        if (Array.isArray(data)) {
            const allOps = new Map();

            for (const snapshot of data) {
                if (snapshot.ops) {
                    for (const op of snapshot.ops) {
                        if (!allOps.has(op.id)) {
                            allOps.set(op.id, op);
                        }
                    }
                }
            }

            const sortedOps = Array.from(allOps.values()).sort((a, b) => a.id - b.id);

            return {
                cycle: data[data.length - 1]?.cycle || 0,
                committed_count: data[data.length - 1]?.committed_count || 0,
                ops: sortedOps,
                metadata: {}
            };
        }

        // Return as-is if we can't normalize
        return data;
    }

    updateStatus(message) {
        if (this.elements.statusBar) {
            this.elements.statusBar.textContent = message;
        }
        console.log('Status:', message);
    }
}

// Initialize when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    window.staticViz = new StaticVisualization();
});
