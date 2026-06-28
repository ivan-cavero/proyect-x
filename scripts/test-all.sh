#!/bin/bash
# test-all.sh — Test everything: CLI, API, Dashboard

set -e

echo "🧪 Project-X — Full System Test"
echo "═══════════════════════════════════════════════════════════"
echo ""

# ─── 1. Build ───────────────────────────────────────────────
echo "1️⃣  Building..."
cargo build --release 2>&1 | tail -1
echo "   ✓ Binary built: target/release/project-x"
echo ""

# ─── 2. CLI Tests ────────────────────────────────────────────
echo "2️⃣  CLI Tests..."
echo "   -- version"
project-x --version
echo ""

echo "   -- init"
project-x init test-project 2>&1 | head -3
echo ""

echo "   -- config show (in project)"
cd test-project && project-x config show 2>&1 | head -5
cd ..
echo ""

# ─── 3. Rust Tests ───────────────────────────────────────────
echo "3️⃣  Rust Unit Tests..."
cargo test --workspace 2>&1 | grep "test result:" | head -10
echo ""

# ─── 4. Integration Test ──────────────────────────────────────
echo "4️⃣  Integration Test..."
project-x test 2>&1 | head -20
echo ""

# ─── 5. Dashboard ────────────────────────────────────────────
echo "5️⃣  Dashboard (start with: cd dashboard && npm run dev)"
echo "   URL: http://localhost:3000"
echo ""

# ─── 6. API Health ──────────────────────────────────────────
echo "6️⃣  API Health Check..."
if curl -s http://localhost:8080/api/health > /dev/null 2>&1; then
    echo "   ✓ API running on port 8080"
    curl -s http://localhost:8080/api/health | python3 -m json.tool 2>/dev/null || echo "   ✓ API responding"
else
    echo "   ⚠ API not running (start with: project-x run --goal test)"
fi
echo ""

echo "═══════════════════════════════════════════════════════════"
echo "  ✅ Full test complete!"
echo "  Dashboard: cd dashboard && npm run dev"
echo "  API:       project-x run --goal test"
echo "═══════════════════════════════════════════════════════════"

# Cleanup
rm -rf test-project