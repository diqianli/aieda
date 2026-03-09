#!/bin/bash
# Download SPEC CPU 2017 traces from ChampSim for validation
#
# ChampSim provides pre-generated traces at:
# http://hpca23.cse.tamu.edu/champsim-traces/speccpu/

TRACE_DIR="traces"
BASE_URL="http://hpca23.cse.tamu.edu/champsim-traces/speccpu"

mkdir -p "$TRACE_DIR"

echo "SPEC CPU 2017 Trace Downloader"
echo "==============================="
echo ""
echo "Downloading traces to: $TRACE_DIR"
echo ""

# List of commonly used SPEC17 traces
TRACES=(
    "600.perlbench_s-210B.champsimtrace.xz"
    "603.bwaves_s-210B.champsimtrace.xz"
    "607.cactuBSSN_s-210B.champsimtrace.xz"
    "619.lbm_s-210B.champsimtrace.xz"
    "620.omnetpp_s-210B.champsimtrace.xz"
    "621.wrf_s-210B.champsimtrace.xz"
    "623.xalancbmk_s-210B.champsimtrace.xz"
    "625.x264_s-210B.champsimtrace.xz"
    "631.deepsjeng_s-210B.champsimtrace.xz"
    "638.imagick_s-210B.champsimtrace.xz"
    "641.leela_s-210B.champsimtrace.xz"
    "648.exchange2_s-210B.champsimtrace.xz"
    "657.xz_s-210B.champsimtrace.xz"
)

echo "Available traces:"
echo "  1. 600.perlbench_s (Perl interpreter)"
echo "  2. 603.bwaves_s (Fluid dynamics)"
echo "  3. 607.cactuBSSN_s (Physics simulation)"
echo "  4. 619.lbm_s (Fluid dynamics)")
echo "  5. 620.omnetpp_s (Discrete event simulation)"
echo "  6. 621.wrf_s (Weather prediction)")
echo "  7. 623.xalancbmk_s (XML processing)")
echo "  8. 625.x264_s (Video encoding)")
echo "  9. 631.deepsjeng_s (Chess engine)")
echo "  10. 638.imagick_s (Image processing)")
echo "  11. 641.leela_s (Go engine)")
echo "  12. 648.exchange2_s (Exchange simulation)")
echo "  13. 657.xz_s (Data compression)")
echo ""

if [ -z "$1" ]; then
    echo "Usage: $0 <trace_number|all|list>"
    echo ""
    echo "Examples:"
    echo "  $0 1        # Download 600.perlbench_s"
    echo "  $0 all      # Download all traces"
    echo "  $0 list     # Just list available traces"
    exit 0
fi

if [ "$1" == "list" ]; then
    exit 0
fi

download_trace() {
    local trace=$1
    local url="$BASE_URL/$trace"
    local output="$TRACE_DIR/$trace"

    if [ -f "$output" ]; then
        echo "  [SKIP] $trace (already exists)"
        return
    fi

    echo "  [DOWN] $trace"
    wget -q --show-progress -O "$output" "$url"

    if [ $? -eq 0 ]; then
        echo "  [DONE] $trace"
    else
        echo "  [FAIL] $trace"
        rm -f "$output"
    fi
}

if [ "$1" == "all" ]; then
    echo "Downloading all traces..."
    echo ""
    for trace in "${TRACES[@]}"; do
        download_trace "$trace"
    done
else
    idx=$(($1 - 1))
    if [ $idx -ge 0 ] && [ $idx -lt ${#TRACES[@]} ]; then
        trace="${TRACES[$idx]}"
        echo "Downloading: $trace"
        download_trace "$trace"
    else
        echo "Invalid trace number: $1"
        exit 1
    fi
fi

echo ""
echo "Download complete!"
echo ""
echo "To validate the emulator with a trace, run:"
echo "  cargo run --example spec17_validation -- traces/<trace_file> 1000000"
