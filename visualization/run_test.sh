#!/bin/bash
# Konata Renderer Auto Test Script
# Automatically starts HTTP server and runs Playwright tests

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo "🚀 Konata Renderer Auto Test"
echo "=============================="

# Kill any existing servers on port 8080
pkill -f "python3 -m http.server 8080" 2>/dev/null || true
sleep 1

# Start HTTP server in background
echo "📡 Starting HTTP server on port 8080..."
nohup python3 -m http.server 8080 > /tmp/konata_server.log 2>&1 &
SERVER_PID=$!
sleep 2

# Verify server is running
if curl -s http://localhost:8080/test_konata.html > /dev/null; then
    echo "✅ HTTP server started (PID: $SERVER_PID)"
else
    echo "❌ Failed to start HTTP server"
    exit 1
fi

# Run Playwright tests
echo ""
echo "🧪 Running Playwright tests..."
echo "------------------------------"
node tests/konata_test.mjs

# Capture exit code
TEST_EXIT_CODE=$?

# Cleanup
echo ""
echo "🧹 Cleaning up..."
kill $SERVER_PID 2>/dev/null || true

# Open screenshots directory
echo ""
echo "📸 Opening screenshots directory..."
open test_screenshots 2>/dev/null || echo "Screenshots saved to: $SCRIPT_DIR/test_screenshots/"

exit $TEST_EXIT_CODE
