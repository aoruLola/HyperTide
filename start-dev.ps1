# HyperTide 开发环境启动脚本

Write-Host "🌊 启动 HyperTide 开发环境..." -ForegroundColor Cyan
Write-Host ""

# 启动后端服务
Write-Host "📦 启动后端服务 (Rust)..." -ForegroundColor Yellow
Start-Process pwsh -ArgumentList "-NoExit", "-Command", "cargo run" -WorkingDirectory $PSScriptRoot

# 等待后端启动
Write-Host "⏳ 等待后端服务启动..." -ForegroundColor Yellow
Start-Sleep -Seconds 3

# 启动前端应用
Write-Host "🎨 启动前端应用 (Tauri + React)..." -ForegroundColor Yellow
Set-Location "$PSScriptRoot\hypertide-ui"
npm run tauri dev
