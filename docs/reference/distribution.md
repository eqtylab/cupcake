# Cupcake Distribution Strategy

## Vision

Cupcake should be accessible to developers through their preferred installation method, meeting them where they already have momentum. This document outlines our comprehensive distribution strategy across multiple platforms and package managers.

## Current Status (v0.2.0)

✅ **Phase 1 & 2 Complete**
- GitHub Releases with pre-built binaries
- Universal install scripts (shell & PowerShell)
- Multi-platform support (macOS, Linux, Windows)
- **NEW: OPA bundled in releases (batteries included)**

## OPA Bundling Strategy

Starting with v0.2.0, Cupcake bundles the Open Policy Agent (OPA) v1.7.1 binary directly in release artifacts. This eliminates the need for users to separately install OPA, providing a true "batteries included" experience.

### Implementation Details

- **OPA Version**: v1.7.1 (supports Rego v1.0 syntax)
- **Bundle Size**: ~45-70MB OPA binary increases release size from ~10MB to ~55-80MB total
- **Platform Matching**: Each platform gets its appropriate OPA binary (static builds preferred)
- **Checksum Verification**: OPA binaries are verified during the build process
- **Lookup Order**:
  1. `--opa-path` CLI flag (if specified)
  2. Bundled OPA (same directory as cupcake binary)
  3. System PATH (fallback)

### Benefits

- **Zero Friction**: Works immediately after installation
- **Version Consistency**: Guaranteed compatible OPA version  
- **Corporate Friendly**: No runtime downloads required
- **Simple Distribution**: Single archive contains everything

## Installation Methods

### Available Now

#### Direct Download
```bash
# Universal installer (macOS/Linux)
curl -fsSL https://raw.githubusercontent.com/eqtylab/cupcake/main/scripts/install.sh | sh

# Windows PowerShell
irm https://raw.githubusercontent.com/eqtylab/cupcake/main/scripts/install.ps1 | iex
```

#### GitHub Releases
Download pre-built binaries directly from [GitHub Releases](https://github.com/eqtylab/cupcake/releases).

### Planned Distribution Channels

#### Phase 3: Language Packages (Q1 2025)

**Python (PyPI)**
```bash
pip install cupcake
```
- Full CLI + Python API
- Platform-specific wheels via maturin
- No compilation required

**Node.js (npm)**
```bash
npm install -g @cupcake/cli
```
- Downloads platform binary on install
- Both CLI and programmatic API
- TypeScript definitions included

#### Phase 4: Platform Integration (Q2 2025)

**macOS (Homebrew)**
```bash
brew install cupcake
```
- Start with tap: `brew tap cupcake/tap`
- Graduate to homebrew-core after stability

**Windows (winget)**
```powershell
winget install cupcake
```
- MSI installer for traditional installation
- Portable ZIP for automation

**Linux Package Managers**
- **apt/deb**: Ubuntu/Debian support
- **yum/rpm**: RHEL/Fedora support
- **AUR**: Arch Linux (community maintained)
- **snap**: Universal Linux packages

## Platform Support Matrix

| Platform | Architecture | Status | Archive Size |
|----------|-------------|--------|--------------|
| macOS | x86_64 | ✅ Ready | ~70MB |
| macOS | aarch64 | ✅ Ready | ~60MB |
| Linux | x86_64 | ✅ Ready | ~65MB |
| Linux | aarch64 | ✅ Ready | ~60MB |
| Linux | x86_64-musl | ✅ Ready | ~65MB |
| Windows | x86_64 | ✅ Ready | ~75MB |
| Windows | aarch64 | 🔄 Planned | ~75MB |

## Build Infrastructure

### GitHub Actions Workflow

Our release process is fully automated via GitHub Actions:

1. **Trigger**: Push tag matching `v*` pattern
2. **Build**: Parallel builds for all platforms
3. **Package**: Create archives with checksums
4. **Release**: Upload to GitHub Releases
5. **Distribute**: Push to package registries

### Build Optimization

We use a custom `dist` profile for releases:
- Link-time optimization (LTO)
- Single codegen unit
- Symbol stripping
- Panic abort for smaller binaries

## Security & Trust

### Artifact Signing
- SHA256 checksums for all artifacts
- Signed commits and tags
- SLSA provenance (planned)
- Sigstore/Cosign integration (planned)

### Verification
All installers verify checksums before installation. Manual verification:
```bash
# Download checksum file
curl -LO https://github.com/eqtylab/cupcake/releases/download/v0.1.0/SHA256SUMS

# Verify your download
sha256sum -c SHA256SUMS --ignore-missing
```

## External Dependencies

### OPA (Bundled)
Starting with v0.2.0, OPA v1.7.1 is bundled with Cupcake. No separate installation required!

The bundled OPA:
- Located in the same directory as the cupcake binary
- Automatically discovered and used by cupcake
- Can be overridden with `--opa-path` CLI flag

## Version Management

### Versioning Scheme
We follow Semantic Versioning (SemVer):
- **Major**: Breaking changes to CLI or policy format
- **Minor**: New features, backward compatible
- **Patch**: Bug fixes and minor improvements

### Release Channels
- **Stable**: Production-ready releases
- **Beta**: Pre-release testing (opt-in)
- **Nightly**: Automated builds from main (planned)

## Community Packages

We welcome community-maintained packages. Guidelines:
1. Use official binaries from GitHub Releases
2. Verify checksums during packaging
3. Preserve LICENSE and attribution
4. Submit PRs to update this document

## Support Matrix

| Distribution Method | Support Level | Maintainer |
|--------------------|---------------|------------|
| GitHub Releases | Official | Core Team |
| Install Scripts | Official | Core Team |
| PyPI | Official | Core Team |
| npm | Official | Core Team |
| Homebrew | Official | Core Team |
| winget | Community | Contributors |
| Linux Packages | Community | Distro Maintainers |

## Contributing

To add a new distribution channel:
1. Open an issue describing the channel
2. Implement packaging following our guidelines
3. Add automated testing
4. Update this documentation
5. Submit PR for review

## Metrics & Goals

### Success Metrics
- Install success rate: >95%
- Time to first success: <2 minutes
- Platform coverage: >90% of developers
- Support burden: <5% of users

### Q1 2025 Goals
- 10,000+ downloads
- 5+ distribution channels
- <1 minute average install time
- Zero-dependency installation option

## FAQ

### Why so many distribution methods?
Different developers have different workflows. By supporting multiple channels, we reduce friction and meet developers where they are.

### Which method should I use?
- **Python developers**: Use pip
- **Node.js developers**: Use npm
- **macOS users**: Use Homebrew
- **CI/CD pipelines**: Use direct download
- **Enterprises**: Use platform packages

### How do I update Cupcake?
Each distribution method has its own update mechanism:
- pip: `pip install --upgrade cupcake`
- npm: `npm update -g @cupcake/cli`
- brew: `brew upgrade cupcake`
- Direct: Re-run install script

### Is Cupcake available in containers?
Yes! Official Docker images are planned for Phase 4. For now, use multi-stage builds with our Linux binaries.

## Contact

- **Issues**: [GitHub Issues](https://github.com/eqtylab/cupcake/issues)
- **Discussions**: [GitHub Discussions](https://github.com/eqtylab/cupcake/discussions)
- **Security**: security@cupcake.dev (planned)