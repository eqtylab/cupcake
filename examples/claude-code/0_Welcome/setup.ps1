# PowerShell script for Windows setup
# Run with: powershell -ExecutionPolicy Bypass -File setup.ps1

Write-Host "Cupcake Evaluation Setup (Windows)" -ForegroundColor Green
Write-Host "==================================`n"

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

# Initialize Cupcake project
Write-Host "`nInitializing Cupcake project..."
cupcake init
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Project initialization failed" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Project initialized" -ForegroundColor Green

# Copy example policies to Claude Code policies directory
Write-Host "`nCopying example policies..."
Copy-Item -Path "..\..\fixtures\security_policy.rego" -Destination ".cupcake\policies\claude\" -Force
Copy-Item -Path "..\..\fixtures\git_workflow.rego" -Destination ".cupcake\policies\claude\" -Force
Copy-Item -Path "..\..\fixtures\context_injection.rego" -Destination ".cupcake\policies\claude\" -Force
Write-Host "✅ Example policies copied" -ForegroundColor Green

Write-Host "✅ Builtins configured (protected_paths, git_pre_check, rulebook_security_guardrails)" -ForegroundColor Green

# Compile policies to WASM (only Claude Code policies)
Write-Host "`nCompiling Claude Code policies to WASM..."
opa build -t wasm -e cupcake/system/evaluate .cupcake/policies/claude/
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Policy compilation failed" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Policies compiled to bundle.tar.gz" -ForegroundColor Green

# Create Claude Code settings directory and hooks integration
Write-Host "`nSetting up Claude Code hooks integration..."
New-Item -ItemType Directory -Force -Path ".claude" | Out-Null

# Get absolute paths
$manifestPath = Resolve-Path "..\..\..\Cargo.toml"
$opaDir = Split-Path (Get-Command opa).Source -Parent

# Create Claude Code settings with Windows paths
$settingsContent = @"
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "cargo run --manifest-path `"$manifestPath`" -- eval --harness claude --log-level info",
            "timeout": 120,
            "env": {
              "PATH": "$opaDir;%PATH%"
            }
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "cargo run --manifest-path `"$manifestPath`" -- eval --harness claude --log-level info",
            "timeout": 120,
            "env": {
              "PATH": "$opaDir;%PATH%"
            }
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cargo run --manifest-path `"$manifestPath`" -- eval --harness claude --log-level info",
            "timeout": 120,
            "env": {
              "PATH": "$opaDir;%PATH%"
            }
          }
        ]
      }
    ]
  }
}
"@

$settingsContent | Out-File -FilePath ".claude\settings.json" -Encoding UTF8
Write-Host "✅ Claude Code hooks configured" -ForegroundColor Green

Write-Host "`n🎉 Setup complete!" -ForegroundColor Green
Write-Host "`nNext steps:" -ForegroundColor Cyan
Write-Host "1. Add cupcake to your PATH:" -ForegroundColor White
Write-Host "   `$env:PATH = `"$(Resolve-Path ..\..\..\target\release);`$env:PATH`"" -ForegroundColor Yellow
Write-Host "2. Start Claude Code in this directory" -ForegroundColor White
Write-Host "3. Try running commands that trigger policies" -ForegroundColor White
Write-Host "`nExample commands to test:" -ForegroundColor Cyan
Write-Host "- ls (safe, should work)" -ForegroundColor White
Write-Host "- Remove-Item -Recurse -Force C:\temp\test (dangerous, should block)" -ForegroundColor White
Write-Host "- Edit C:\Windows\System32\drivers\etc\hosts (system file, should block)" -ForegroundColor White
Write-Host "- git push --force (risky, should ask)" -ForegroundColor White