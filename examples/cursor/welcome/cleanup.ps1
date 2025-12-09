# PowerShell cleanup script for Windows
# Run with: powershell -ExecutionPolicy Bypass -File cleanup.ps1

Write-Host "Cupcake Cursor Evaluation Cleanup (Windows)" -ForegroundColor Green
Write-Host "===========================================`n"

# Remove Cupcake project files
if (Test-Path ".cupcake") {
    Write-Host "Removing .cupcake directory..."
    Remove-Item -Path ".cupcake" -Recurse -Force
    Write-Host "✅ .cupcake directory removed" -ForegroundColor Green
}

# Remove Cursor hooks
if (Test-Path ".cursor") {
    Write-Host "Removing .cursor directory..."
    Remove-Item -Path ".cursor" -Recurse -Force
    Write-Host "✅ .cursor directory removed" -ForegroundColor Green
}

# Remove compiled bundle
if (Test-Path "bundle.tar.gz") {
    Write-Host "Removing compiled bundle..."
    Remove-Item -Path "bundle.tar.gz" -Force
    Write-Host "✅ Bundle removed" -ForegroundColor Green
}

Write-Host "`n🧹 Cleanup complete!" -ForegroundColor Green
Write-Host "`nRun .\setup.ps1 to reinitialize the evaluation environment." -ForegroundColor Cyan