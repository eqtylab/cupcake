# PowerShell script for Windows setup
# Run with: powershell -ExecutionPolicy Bypass -File setup.ps1

Write-Host "Cupcake Cursor Evaluation Setup (Windows)" -ForegroundColor Green
Write-Host "=========================================`n"

# Check if Rust/Cargo is installed
try {
    $cargoVersion = cargo --version 2>$null
    Write-Host "✅ Cargo found: $cargoVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ Cargo not found in PATH. Please install Rust:" -ForegroundColor Red
    Write-Host "   https://rustup.rs/" -ForegroundColor Yellow
    exit 1
}

# Check if OPA is installed
try {
    $opaVersion = opa version 2>$null | Select-Object -First 1
    Write-Host "✅ OPA found: $opaVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ OPA not found in PATH. Please install OPA:" -ForegroundColor Red
    Write-Host "   https://www.openpolicyagent.org/docs/latest/#running-opa" -ForegroundColor Yellow
    Write-Host "   For Windows: Download opa_windows_amd64.exe and rename to opa.exe" -ForegroundColor Yellow
    exit 1
}

# Build Cupcake binary
Write-Host "`nBuilding Cupcake binary..."
Push-Location ../../..
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Build failed" -ForegroundColor Red
    Pop-Location
    exit 1
}
Write-Host "✅ Build complete" -ForegroundColor Green

# Add to PATH for this session
$cupcakePath = Join-Path (Get-Location) "target\release"
$env:PATH = "$cupcakePath;$env:PATH"
Write-Host "✅ Added cupcake to PATH for this session" -ForegroundColor Green

# Return to eval directory
Pop-Location

# Initialize Cupcake project with Cursor harness
Write-Host "`nInitializing Cupcake project for Cursor..."
cupcake init --harness cursor
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Project initialization failed" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Project initialized with Cursor harness" -ForegroundColor Green

# Create Cursor-specific policies
Write-Host "`nCreating Cursor-specific policies..."

# Create security policy
$securityPolicy = @'
# METADATA
# scope: package
# title: Cursor Security Policy
# custom:
#   routing:
#     required_events: ["beforeShellExecution"]
package cupcake.policies.cursor.security

import rego.v1

# Block dangerous shell commands with differentiated feedback
deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    dangerous_commands := ["Remove-Item -Recurse -Force", "rm -rf", "del /f /s /q", "format", "diskpart"]
    some cmd in dangerous_commands
    contains(lower(input.command), lower(cmd))

    decision := {
        "rule_id": "CURSOR-SECURITY-001",
        "reason": concat(" ", ["Dangerous command blocked:", cmd]),
        "agent_context": concat("", [
            cmd, " detected in command. This is a destructive operation. ",
            "Alternatives: 1) Use Recycle Bin for safe deletion, ",
            "2) Be more specific with paths, ",
            "3) Use -WhatIf flag first to preview changes."
        ]),
        "severity": "CRITICAL"
    }
}

# Block admin operations
deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    admin_indicators := ["Run as administrator", "runas", "elevate", "admin"]
    some indicator in admin_indicators
    contains(lower(input.command), lower(indicator))

    decision := {
        "rule_id": "CURSOR-ADMIN-001",
        "reason": "Administrative privileges required",
        "agent_context": "Admin operation detected. Elevated privileges are dangerous. Consider: 1) Use specific commands without admin rights, 2) Modify permissions instead, 3) Use containers for isolation.",
        "severity": "HIGH"
    }
}
'@

$securityPolicy | Out-File -FilePath ".cupcake\policies\cursor\security_policy.rego" -Encoding UTF8

# Create file protection policy
$fileProtectionPolicy = @'
# METADATA
# scope: package
# title: Cursor File Protection
# custom:
#   routing:
#     required_events: ["beforeReadFile", "afterFileEdit"]
package cupcake.policies.cursor.file_protection

import rego.v1

# Protect sensitive files from reading
deny contains decision if {
    input.hook_event_name == "beforeReadFile"
    sensitive_patterns := [".ssh\\id_", ".aws\\credentials", ".env", "secrets", "AppData\\Local\\Google\\Chrome"]
    some pattern in sensitive_patterns
    contains(lower(input.file_path), lower(pattern))

    decision := {
        "rule_id": "CURSOR-FILE-READ-001",
        "reason": "Access to sensitive file blocked",
        "agent_context": concat("", [
            "Attempted to read sensitive file containing '", pattern, "'. ",
            "These files contain secrets that should not be exposed. ",
            "Instead: 1) Ask user to provide redacted version, ",
            "2) Use environment variables, ",
            "3) Create example/template files without real secrets."
        ]),
        "severity": "CRITICAL"
    }
}

# Validate system file edits
deny contains decision if {
    input.hook_event_name == "afterFileEdit"
    system_paths := ["C:\\Windows\\", "C:\\Program Files\\", "C:\\ProgramData\\"]
    some path in system_paths
    contains(lower(input.file_path), lower(path))

    decision := {
        "rule_id": "CURSOR-FILE-EDIT-001",
        "reason": "System file modification blocked",
        "agent_context": "Attempted to modify system file. System files require manual intervention. Create configuration in user space instead.",
        "severity": "HIGH"
    }
}
'@

$fileProtectionPolicy | Out-File -FilePath ".cupcake\policies\cursor\file_protection.rego" -Encoding UTF8

Write-Host "✅ Cursor-specific policies created" -ForegroundColor Green

# Compile policies to WASM (only Cursor policies)
Write-Host "`nCompiling Cursor policies to WASM..."
opa build -t wasm -e cupcake/system/evaluate .cupcake/policies/cursor/
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Policy compilation failed" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Policies compiled to bundle.tar.gz" -ForegroundColor Green

# Create Cursor hooks configuration
Write-Host "`nSetting up Cursor hooks integration..."
$cupcakeExe = Join-Path (Resolve-Path "..\..\..\target\release") "cupcake.exe"
$hooksDir = Join-Path $env:USERPROFILE ".cursor"
$hooksFile = Join-Path $hooksDir "hooks.json"

# Create .cursor directory if it doesn't exist
if (!(Test-Path $hooksDir)) {
    New-Item -ItemType Directory -Force -Path $hooksDir | Out-Null
}

# Check if hooks.json already exists
if (Test-Path $hooksFile) {
    Write-Host "⚠️  Existing hooks.json found. Creating backup..." -ForegroundColor Yellow
    $backupName = "hooks.json.backup.$(Get-Date -Format 'yyyyMMdd_HHmmss')"
    Copy-Item -Path $hooksFile -Destination (Join-Path $hooksDir $backupName)
}

# Create Cursor hooks configuration
$hooksContent = @"
{
  "version": 1,
  "hooks": {
    "beforeShellExecution": [
      {
        "command": "`"$cupcakeExe`" eval --harness cursor --log-level info"
      }
    ],
    "beforeMCPExecution": [
      {
        "command": "`"$cupcakeExe`" eval --harness cursor --log-level info"
      }
    ],
    "afterFileEdit": [
      {
        "command": "`"$cupcakeExe`" eval --harness cursor --log-level info"
      }
    ],
    "beforeReadFile": [
      {
        "command": "`"$cupcakeExe`" eval --harness cursor --log-level info"
      }
    ],
    "beforeSubmitPrompt": [
      {
        "command": "`"$cupcakeExe`" eval --harness cursor --log-level info"
      }
    ],
    "stop": [
      {
        "command": "`"$cupcakeExe`" eval --harness cursor --log-level info"
      }
    ]
  }
}
"@

$hooksContent | Out-File -FilePath $hooksFile -Encoding UTF8
Write-Host "✅ Cursor hooks configured at $hooksFile" -ForegroundColor Green

# Create test events directory
New-Item -ItemType Directory -Force -Path "test-events" | Out-Null

# Create test event files
$shellRmEvent = @'
{
  "hook_event_name": "beforeShellExecution",
  "conversation_id": "test-001",
  "generation_id": "gen-001",
  "workspace_roots": ["C:\\temp"],
  "command": "Remove-Item -Recurse -Force C:\\temp\\test",
  "cwd": "C:\\temp"
}
'@

$shellRmEvent | Out-File -FilePath "test-events\shell-rm.json" -Encoding UTF8

$fileReadEvent = @'
{
  "hook_event_name": "beforeReadFile",
  "conversation_id": "test-002",
  "generation_id": "gen-002",
  "workspace_roots": ["C:\\Users\\user"],
  "file_path": "C:\\Users\\user\\.ssh\\id_rsa",
  "file_content": "-----BEGIN OPENSSH PRIVATE KEY-----",
  "attachments": []
}
'@

$fileReadEvent | Out-File -FilePath "test-events\file-read-ssh.json" -Encoding UTF8

Write-Host "✅ Test events created in test-events\" -ForegroundColor Green

# Create screenshots directory
New-Item -ItemType Directory -Force -Path "screenshots" | Out-Null
Write-Host "📸 Screenshots directory created (placeholder for demo screenshots)" -ForegroundColor Green

Write-Host "`n🎉 Setup complete!" -ForegroundColor Green
Write-Host "`nNext steps:" -ForegroundColor Cyan
Write-Host "1. Restart Cursor to load the new hooks configuration" -ForegroundColor White
Write-Host "2. Open this directory in Cursor" -ForegroundColor White
Write-Host "3. Try commands that trigger policies:" -ForegroundColor White
Write-Host "   - 'delete C:\temp\test directory' (blocks Remove-Item -Force)" -ForegroundColor White
Write-Host "   - 'read my SSH key' (blocks sensitive file access)" -ForegroundColor White
Write-Host "   - 'run as administrator' (blocks admin operations)" -ForegroundColor White
Write-Host "`nTest policies manually:" -ForegroundColor Cyan
Write-Host "Get-Content test-events\shell-rm.json | cupcake eval --harness cursor" -ForegroundColor Yellow
Write-Host "`nView active policies:" -ForegroundColor Cyan
Write-Host "cupcake inspect --harness cursor" -ForegroundColor Yellow