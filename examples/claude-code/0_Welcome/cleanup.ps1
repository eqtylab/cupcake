# PowerShell cleanup script for Windows
# Run with: powershell -ExecutionPolicy Bypass -File cleanup.ps1

Write-Host "Cupcake Evaluation Cleanup (Windows)" -ForegroundColor Green
Write-Host "====================================`n"

# Remove Cupcake project files
if (Test-Path ".cupcake") {
    Write-Host "Removing .cupcake directory..."
    Remove-Item -Path ".cupcake" -Recurse -Force
    Write-Host "✅ .cupcake directory removed" -ForegroundColor Green
}

# Remove Claude Code settings
if (Test-Path ".claude") {
    Write-Host "Removing .claude directory..."
    Remove-Item -Path ".claude" -Recurse -Force
    Write-Host "✅ .claude directory removed" -ForegroundColor Green
}

# Remove compiled bundle
if (Test-Path "bundle.tar.gz") {
    Write-Host "Removing compiled bundle..."
    Remove-Item -Path "bundle.tar.gz" -Force
    Write-Host "✅ Bundle removed" -ForegroundColor Green
}

# Remove MCP configuration if exists
if (Test-Path ".mcp") {
    Write-Host "Removing .mcp directory..."
    Remove-Item -Path ".mcp" -Recurse -Force
    Write-Host "✅ MCP configuration removed" -ForegroundColor Green
}

Write-Host "`n🧹 Cleanup complete!" -ForegroundColor Green
Write-Host "`nRun .\setup.ps1 to reinitialize the evaluation environment." -ForegroundColor Cyan