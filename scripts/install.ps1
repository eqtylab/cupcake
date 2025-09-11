# Cupcake Installation Script for Windows
# 
# This script downloads and installs the Cupcake CLI tool on Windows.
# It automatically downloads the appropriate binary from GitHub releases,
# verifies checksums, and installs to your PATH.
#
# Usage:
#   irm https://raw.githubusercontent.com/eqtylab/cupcake/main/scripts/install.ps1 | iex
#   Invoke-WebRequest -Uri https://raw.githubusercontent.com/eqtylab/cupcake/main/scripts/install.ps1 | Invoke-Expression

[CmdletBinding()]
param(
    [string]$Version = "",
    [string]$InstallDir = "$env:USERPROFILE\.cupcake",
    [string]$GithubRepo = "eqtylab/cupcake"
)

$ErrorActionPreference = "Stop"

# Configuration
$BinaryName = "cupcake.exe"
$BinDir = Join-Path $InstallDir "bin"

# Helper functions
function Write-Info {
    param([string]$Message)
    Write-Host $Message -ForegroundColor Blue
}

function Write-Success {
    param([string]$Message)
    Write-Host $Message -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "Warning: $Message" -ForegroundColor Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "Error: $Message" -ForegroundColor Red
    exit 1
}

# Detect architecture
function Get-Architecture {
    $arch = $env:PROCESSOR_ARCHITECTURE
    switch ($arch) {
        "AMD64" { return "x86_64-pc-windows-msvc" }
        "ARM64" { Write-Error "ARM64 Windows is not supported yet" }
        default { Write-Error "Unsupported architecture: $arch" }
    }
}

# Get latest version from GitHub
function Get-LatestVersion {
    try {
        $releases = Invoke-RestMethod -Uri "https://api.github.com/repos/$GithubRepo/releases/latest"
        return $releases.tag_name
    }
    catch {
        Write-Error "Failed to fetch latest version from GitHub: $_"
    }
}

# Download file with progress
function Download-File {
    param(
        [string]$Url,
        [string]$OutputPath
    )
    
    try {
        Write-Info "Downloading from $Url..."
        $ProgressPreference = 'SilentlyContinue'  # Faster downloads
        Invoke-WebRequest -Uri $Url -OutFile $OutputPath -UseBasicParsing
        $ProgressPreference = 'Continue'
    }
    catch {
        Write-Error "Failed to download file: $_"
    }
}

# Verify checksum
function Verify-Checksum {
    param(
        [string]$FilePath,
        [string]$ChecksumPath
    )
    
    # Read the checksum file
    $checksumContent = Get-Content $ChecksumPath -Raw
    $expectedHash = ($checksumContent -split '\s+')[0].ToLower()
    
    # Calculate actual hash
    $actualHash = (Get-FileHash -Path $FilePath -Algorithm SHA256).Hash.ToLower()
    
    if ($expectedHash -ne $actualHash) {
        Write-Error "Checksum verification failed!`nExpected: $expectedHash`nActual: $actualHash"
    }
    
    Write-Success "✓ Checksum verified"
}

# Note: OPA is now bundled with Cupcake
function Test-BundledOpa {
    $opaPath = Join-Path $BinDir "opa.exe"
    if (Test-Path $opaPath) {
        Write-Success "✓ OPA is bundled with Cupcake"
        return $true
    }
    else {
        Write-Warning "OPA binary not found in bundle (this should not happen)"
        return $false
    }
}

# Add to PATH
function Add-ToPath {
    param([string]$Directory)
    
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    
    if ($userPath -notlike "*$Directory*") {
        Write-Info "Adding $Directory to PATH..."
        
        # Add to current session
        $env:Path = "$Directory;$env:Path"
        
        # Add to user PATH permanently
        $newPath = "$Directory;$userPath"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        
        Write-Success "✓ Added to PATH (restart your terminal to use 'cupcake' command)"
    }
    else {
        Write-Info "Already in PATH: $Directory"
    }
}

# Main installation
function Install-Cupcake {
    Write-Host ""
    Write-Info "Installing Cupcake CLI..."
    Write-Host ""
    
    # Detect architecture
    $platform = Get-Architecture
    Write-Info "Platform: $platform"
    
    # Get version
    if ([string]::IsNullOrEmpty($Version)) {
        $Version = Get-LatestVersion
    }
    Write-Info "Version: $Version"
    
    # Create temp directory
    $tempDir = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }
    
    try {
        # Construct download URLs
        $archiveName = "cupcake-$Version-$platform.zip"
        $downloadUrl = "https://github.com/$GithubRepo/releases/download/$Version/$archiveName"
        $checksumUrl = "$downloadUrl.sha256"
        
        # Download archive
        $archivePath = Join-Path $tempDir $archiveName
        Download-File -Url $downloadUrl -OutputPath $archivePath
        
        # Download and verify checksum
        Write-Info "Verifying checksum..."
        $checksumPath = "$archivePath.sha256"
        Download-File -Url $checksumUrl -OutputPath $checksumPath
        Verify-Checksum -FilePath $archivePath -ChecksumPath $checksumPath
        
        # Extract archive
        Write-Info "Extracting archive..."
        $extractPath = Join-Path $tempDir "extracted"
        Expand-Archive -Path $archivePath -DestinationPath $extractPath -Force
        
        # Find the binary
        $extractedDir = Get-ChildItem -Path $extractPath -Directory | Select-Object -First 1
        $binarySource = Join-Path $extractedDir.FullName "bin\$BinaryName"
        
        if (-not (Test-Path $binarySource)) {
            Write-Error "Binary not found in archive: $binarySource"
        }
        
        # Create installation directory
        Write-Info "Installing to $BinDir..."
        New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
        
        # Copy binary
        $binaryDest = Join-Path $BinDir $BinaryName
        Copy-Item -Path $binarySource -Destination $binaryDest -Force
        
        # Copy bundled OPA
        $opaSource = Join-Path $extractedDir.FullName "bin\opa.exe"
        if (Test-Path $opaSource) {
            $opaDest = Join-Path $BinDir "opa.exe"
            Copy-Item -Path $opaSource -Destination $opaDest -Force
        }
        
        # Verify installation
        Write-Info "Verifying installation..."
        $testOutput = & $binaryDest --version 2>&1
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Installation verification failed"
        }
        
        Write-Success "✓ Cupcake installed successfully!"
        Write-Host ""
        Write-Host $testOutput
        Write-Host ""
        
        # Add to PATH
        Add-ToPath -Directory $BinDir
        
        # Check for bundled OPA
        Write-Host ""
        Test-BundledOpa | Out-Null
        
        Write-Host ""
        Write-Success "Installation complete!"
        Write-Host ""
        Write-Host "Get started with:" -ForegroundColor Cyan
        Write-Host "  cupcake init        # Initialize a new project"
        Write-Host "  cupcake --help      # Show available commands"
        Write-Host ""
        Write-Host "Documentation: https://github.com/$GithubRepo" -ForegroundColor Cyan
        Write-Host ""
        
        # Windows-specific note
        Write-Host "Note: You may need to restart your terminal or run 'refreshenv' to use the 'cupcake' command." -ForegroundColor Yellow
    }
    finally {
        # Cleanup
        Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# Check if running as administrator (not required, but note if not)
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")
if (-not $isAdmin) {
    Write-Info "Running as user (not administrator). Installing to user directory."
}

# Run installation
Install-Cupcake