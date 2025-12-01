---
layout: "@/layouts/mdx-layout.astro"
heading: "Installation"
description: "Install Cupcake on your system"
---

## Prerequisites

Cupcake requires **Open Policy Agent (OPA)** to compile and evaluate policies. Install OPA before using Cupcake.

### Install OPA

#### macOS

```bash
brew install opa
```

Or download directly:

```bash
# Apple Silicon (M1/M2/M3)
curl -L -o opa https://openpolicyagent.org/downloads/latest/opa_darwin_arm64

# Intel Macs
curl -L -o opa https://openpolicyagent.org/downloads/latest/opa_darwin_amd64

chmod 755 opa
sudo mv opa /usr/local/bin/
```

#### Linux

```bash
# AMD64
curl -L -o opa https://openpolicyagent.org/downloads/latest/opa_linux_amd64

# ARM64
curl -L -o opa https://openpolicyagent.org/downloads/latest/opa_linux_arm64

chmod 755 opa
sudo mv opa /usr/local/bin/
```

#### Windows (PowerShell)

```powershell
Invoke-WebRequest -Uri "https://openpolicyagent.org/downloads/latest/opa_windows_amd64.exe" -OutFile "opa.exe"

# Add to PATH (run as Administrator or add to user PATH)
mkdir C:\Tools\OPA
move opa.exe C:\Tools\OPA\
[Environment]::SetEnvironmentVariable("Path", "$env:Path;C:\Tools\OPA", "User")
```

#### Verify OPA Installation

```bash
opa version
```

You should see output like `Version: 1.11.0` or similar.

For more installation options including Docker, see the [OPA documentation](https://www.openpolicyagent.org/docs#1-download-opa).

## Quick Install

Install Cupcake using the official install scripts:

### Unix/macOS

```bash
curl -fsSL https://get.eqtylab.io/cupcake | bash
```

### Windows PowerShell

```powershell
irm https://get.eqtylab.io/cupcake | iex
```

The install scripts will:

- Download the appropriate binary for your platform
- Verify checksums for security
- Install to your system PATH
- Set up the `cupcake` command globally

## Manual Installation

If you prefer to install manually or need a specific version, you can download pre-built binaries of the [latest release from GitHub](https://github.com/eqtylab/cupcake/releases/latest).

### Install Steps

1. Download the archive for your platform
2. Verify the checksum (optional but recommended): `sha256sum -c cupcake-v0.2.0-<platform>.tar.gz.sha256`
3. Extract the archive: `tar -xzf cupcake-v0.2.0-<platform>.tar.gz` or if using Windows (PowerShell) `Expand-Archive cupcake-v0.2.0-<platform>.zip`
4. Move the binary to your PATH: `sudo mv cupcake /usr/local/bin/` or to a directory in your PATH `mv cupcake ~/.local/bin/`

## Verify Installation

After installation, verify that Cupcake is working:

```bash
cupcake --version
```

You should see output like:

```txt
cupcake 0.2.0
```

## Next Steps

Once installed, you can:

- Initialize a new Cupcake project: `cupcake init`
- Evaluate policies: `cupcake eval`
- Check out the [Usage Guide](usage.md) to get started with policies

## Security

All release binaries and install scripts include SHA256 checksums and are built with [SLSA Level 3](https://slsa.dev/spec/v1.0/levels) compliance. The install scripts themselves are also checksummed (`install.sh.sha256`, `install.ps1.sha256`).

## Troubleshooting

### Command not found

If you get a "command not found" error after installation:

1. Make sure the binary is in a directory that's in your PATH
2. Restart your terminal or run `hash -r` to refresh the PATH cache
3. Check permissions: `chmod +x $(which cupcake)`

### Permission denied

On Unix/macOS, if you get a permission error:

```bash
sudo chmod +x /usr/local/bin/cupcake
```

### macOS Gatekeeper

On macOS, you may need to allow the binary to run:

```bash
xattr -d com.apple.quarantine /usr/local/bin/cupcake
```

Or go to **System Settings → Privacy & Security** and allow the app to run.
