#!/bin/bash
# dev.sh — Start development environment

echo "🚀 Starting Project-X Development Environment"

# Start API server in background
echo "📦 Starting API server on port 8080..."
cargo run --bin project-x -- run --goal "dev-mode" &
API_PID=$!

# Wait for API to start
sleep 2

# Start dashboard
echo "🌐 Starting dashboard on port 3000..."
cd dashboard && npm run dev &
WEB_PID=$!

echo ""
echo "════════════════════════════════════════"
echo "  Dashboard: http://localhost:3000"
echo "  API:       http://localhost:8080/api/health"
echo "  Press Ctrl+C to stop"
echo "════════════════════════════════════════"
echo ""

# Wait for Ctrl+C
trap "echo ''; echo 'Shutting down...'; kill $API_PID $WEB_PID 2>/dev/null; exit 0" SIGINT SIGTERM
wait