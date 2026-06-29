# ═══════════════════════════════════════════════════════════
# Project-X — Launch All Services
# ═══════════════════════════════════════════════════════════
# Usage: .\launch.ps1
#
# Launches:
#   1. Backend API server via project-x.exe (port 8080)
#   2. Frontend dev server via npm (port 3000)
#
# Then opens the dashboard in your default browser.
#
# API keys: manage them via Settings page in the dashboard.
# No env vars needed.

$ErrorActionPreference = "Stop"

$root = $PSScriptRoot
$exe = Join-Path $root "target\release\project-x.exe"

# Fallback to debug if release not built
if (-not (Test-Path $exe)) {
    $exe = Join-Path $root "target\debug\project-x.exe"
}

Write-Host ""
Write-Host "  ╔══════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "  ║   PROJECT-X — Launching All Services     ║" -ForegroundColor Cyan
Write-Host "  ╚══════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""

# ─── 1. Backend ─────────────────────────────────────────
Write-Host "  [1/3] Starting backend API server..." -ForegroundColor Yellow
Write-Host "        Exe: $($exe)" -ForegroundColor Gray
Write-Host "        Port: 8080" -ForegroundColor Gray

$backend = Start-Process -FilePath $exe `
    -ArgumentList "server" `
    -WorkingDirectory $root `
    -PassThru `
    -WindowStyle Hidden

Write-Host "  ✓ Backend started (PID: $($backend.Id))" -ForegroundColor Green
Write-Host ""

# Give backend time to bind
Start-Sleep -Seconds 2

# ─── 2. Frontend ────────────────────────────────────────
Write-Host "  [2/3] Starting frontend dev server..." -ForegroundColor Yellow
Write-Host "        Port: 3000" -ForegroundColor Gray

$dashboard = Join-Path $root "dashboard"
$frontend = Start-Process -FilePath "npm" `
    -ArgumentList "run", "dev" `
    -WorkingDirectory $dashboard `
    -PassThru `
    -WindowStyle Hidden

Write-Host "  ✓ Frontend started (PID: $($frontend.Id))" -ForegroundColor Green
Write-Host ""

# Give frontend time to bind
Start-Sleep -Seconds 2

# ─── 3. Open Browser ────────────────────────────────────
Write-Host "  [3/3] Opening dashboard in browser..." -ForegroundColor Yellow

Start-Process "http://localhost:3000"

Write-Host ""
Write-Host "  ╔══════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "  ║  Dashboard: http://localhost:3000        ║" -ForegroundColor Green
Write-Host "  ║  API:       http://localhost:8080        ║" -ForegroundColor Green
Write-Host "  ║  Health:    http://localhost:8080/api/health" -ForegroundColor Green
Write-Host "  ║  Settings:  Dashboard → Settings tab     ║" -ForegroundColor Green
Write-Host "  ╚══════════════════════════════════════════╝" -ForegroundColor Green
Write-Host ""
Write-Host "  API keys: Manage them in Settings (no env vars needed)" -ForegroundColor Gray
Write-Host "  Press Ctrl+C in this terminal to stop everything." -ForegroundColor Gray
Write-Host ""

# Wait for user to press Ctrl+C
Wait-Process -Id $backend.Id -ErrorAction SilentlyContinue
Write-Host "`n  Backend stopped. Shutting down..." -ForegroundColor Yellow
if ($frontend) {
    Stop-Process -Id $frontend.Id -Force -ErrorAction SilentlyContinue
    Write-Host "  Frontend stopped." -ForegroundColor Yellow
}
Write-Host ""
