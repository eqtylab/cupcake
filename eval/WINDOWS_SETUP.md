# Windows Setup Guide for Cupcake Evaluations

Both Claude Code and Cursor evaluations now include Windows PowerShell scripts alongside the Unix shell scripts.

## Prerequisites for Windows

1. **Install Rust**
   - Download from https://rustup.rs/
   - Follow Windows installation instructions
   - Restart terminal after installation

2. **Install OPA**
   - Go to https://github.com/open-policy-agent/opa/releases
   - Download `opa_windows_amd64.exe`
   - Rename to `opa.exe`
   - Add to PATH (or place in `C:\Windows\System32`)

3. **Install Git for Windows** (optional but recommended)
   - Download from https://git-scm.com/download/win
   - Provides Git Bash for running .sh scripts if preferred

4. **Docker Desktop** (optional, for MCP demos)
   - Download from https://www.docker.com/products/docker-desktop/

## Running the Evaluations

### Option 1: PowerShell (Recommended)

```powershell
# Navigate to evaluation directory
cd eval\0_Claude-Code-Welcome1
# or
cd eval\0_Cursor-Welcome1

# Run setup
powershell -ExecutionPolicy Bypass -File setup.ps1

# Run cleanup when done
powershell -ExecutionPolicy Bypass -File cleanup.ps1
```

### Option 2: Git Bash

If you have Git for Windows installed, you can use the original shell scripts:

```bash
# In Git Bash
cd eval/0_Claude-Code-Welcome1
./setup.sh

# Or for Cursor
cd eval/0_Cursor-Welcome1
./setup.sh
```

## Windows-Specific Paths

### Claude Code
- Settings: `.claude\settings.json` (project directory)
- Global: `%APPDATA%\Claude\settings.json`

### Cursor
- Hooks: `%USERPROFILE%\.cursor\hooks.json`
- Global: `%USERPROFILE%\.cursor\`

## Windows-Specific Policies

The Windows setup scripts create policies that understand Windows paths and commands:

- **PowerShell commands**: `Remove-Item -Recurse -Force`
- **Windows paths**: `C:\Windows\`, `C:\Program Files\`
- **Windows sensitive files**: `AppData\Local\Google\Chrome`

## Testing on Windows

### PowerShell Test Commands

```powershell
# Test policy evaluation
Get-Content test-events\shell-rm.json | cupcake eval --harness cursor

# View active policies
cupcake inspect --harness cursor

# Test dangerous command (will be blocked)
echo '{"hook_event_name":"beforeShellExecution","command":"Remove-Item -Recurse -Force C:\\temp"}' | cupcake eval --harness cursor
```

### CMD Test Commands

```cmd
REM Test policy evaluation
type test-events\shell-rm.json | cupcake eval --harness cursor

REM View active policies
cupcake inspect --harness cursor
```

## Troubleshooting Windows Issues

### OPA Not Found

If OPA is not recognized:
1. Ensure `opa.exe` is in PATH
2. Try using full path: `C:\tools\opa.exe`
3. Restart terminal after adding to PATH

### Cargo Build Fails

If cargo build fails:
1. Ensure Visual Studio Build Tools are installed
2. Run `rustup default stable-msvc`
3. Try building in Developer Command Prompt

### Hooks Not Firing

For Claude Code:
- Check `.claude\settings.json` exists
- Ensure paths use proper escaping (`\\` or `/`)

For Cursor:
- Check `%USERPROFILE%\.cursor\hooks.json`
- Restart Cursor after configuration
- Check Cursor Settings → Hooks tab for errors

### PowerShell Execution Policy

If scripts are blocked:
```powershell
# Temporary bypass (per session)
Set-ExecutionPolicy Bypass -Scope Process

# Or run directly
powershell -ExecutionPolicy Bypass -File setup.ps1
```

## Differences from Unix

1. **Path Separators**: Windows uses `\` (or `/` in many contexts)
2. **Environment Variables**: `%VAR%` in CMD, `$env:VAR` in PowerShell
3. **Commands**: Different utilities (e.g., `Remove-Item` vs `rm`)
4. **File Permissions**: Different permission model than Unix
5. **Shell**: PowerShell/CMD instead of Bash

## Security Notes

- Windows Defender may flag Rust binaries initially
- Add exclusion for `target\release\cupcake.exe` if needed
- Hooks run with user privileges (not admin)
- UAC will block admin operations regardless of policies