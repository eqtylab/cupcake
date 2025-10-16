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

# Remove compiled bundle
if (Test-Path "bundle.tar.gz") {
    Write-Host "Removing compiled bundle..."
    Remove-Item -Path "bundle.tar.gz" -Force
    Write-Host "✅ Bundle removed" -ForegroundColor Green
}

# Remove test events
if (Test-Path "test-events") {
    Write-Host "Removing test-events directory..."
    Remove-Item -Path "test-events" -Recurse -Force
    Write-Host "✅ Test events removed" -ForegroundColor Green
}

# Ask about global hooks cleanup
$hooksFile = Join-Path $env:USERPROFILE ".cursor\hooks.json"

if (Test-Path $hooksFile) {
    Write-Host "`n⚠️  Global Cursor hooks configuration detected at $hooksFile" -ForegroundColor Yellow
    $response = Read-Host "Do you want to remove the global hooks configuration? (y/n)"

    if ($response -eq 'y' -or $response -eq 'Y') {
        # Check for backup
        $hooksDir = Split-Path $hooksFile -Parent
        $latestBackup = Get-ChildItem -Path $hooksDir -Filter "hooks.json.backup.*" |
                        Sort-Object LastWriteTime -Descending |
                        Select-Object -First 1

        if ($latestBackup) {
            Write-Host "Found backup: $($latestBackup.Name)" -ForegroundColor Cyan
            $restoreResponse = Read-Host "Restore from backup? (y/n)"

            if ($restoreResponse -eq 'y' -or $restoreResponse -eq 'Y') {
                Move-Item -Path $latestBackup.FullName -Destination $hooksFile -Force
                Write-Host "✅ Restored hooks.json from backup" -ForegroundColor Green
            } else {
                Remove-Item -Path $hooksFile -Force
                Write-Host "✅ Removed hooks.json (backup preserved)" -ForegroundColor Green
            }
        } else {
            Remove-Item -Path $hooksFile -Force
            Write-Host "✅ Removed hooks.json" -ForegroundColor Green
        }
    } else {
        Write-Host "ℹ️  Keeping global hooks configuration" -ForegroundColor Cyan
    }
}

Write-Host "`n🧹 Cleanup complete!" -ForegroundColor Green
Write-Host "`nRun .\setup.ps1 to reinitialize the evaluation environment." -ForegroundColor Cyan