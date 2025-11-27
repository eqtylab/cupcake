---
layout: "@/layouts/mdx-layout.astro"
heading: "Installation"
description: "Install Cupcake on your system"
---

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
