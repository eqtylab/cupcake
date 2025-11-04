# SLSA Level 3 Verification

Cupcake releases are built with **SLSA Build Level 3** provenance, providing cryptographic proof of:
- **Where** the code came from (source repository and commit)
- **How** it was built (build environment and steps)
- **When** it was built (timestamp)

This ensures supply chain security by making builds verifiable, tamper-evident, and non-forgeable.

## What is SLSA Level 3?

SLSA (Supply-chain Levels for Software Artifacts) Level 3 guarantees:

1. **Non-forgeable provenance**: Signed by GitHub's OIDC infrastructure (not controlled by maintainers)
2. **Build isolation**: Build and signing happen on separate, ephemeral VMs
3. **Ephemeral environment**: Fresh GitHub-hosted runners destroyed after each job

## Verifying Releases

Every Cupcake release includes a `multiple.intoto.jsonl` file containing signed provenance for all platform builds.

### Step 1: Install slsa-verifier

**macOS (Homebrew)**:
```bash
brew install slsa-verifier
```

**Linux/WSL**:
```bash
# Download from releases
LATEST=$(curl -s https://api.github.com/repos/slsa-framework/slsa-verifier/releases/latest | grep tag_name | cut -d '"' -f 4)
curl -Lo slsa-verifier "https://github.com/slsa-framework/slsa-verifier/releases/download/${LATEST}/slsa-verifier-linux-amd64"
chmod +x slsa-verifier
sudo mv slsa-verifier /usr/local/bin/
```

**Verify installation**:
```bash
slsa-verifier version
```

### Step 2: Download Release Assets

Download the artifact you want to verify and the provenance file:

```bash
VERSION="v0.1.0"  # Replace with desired version
PLATFORM="x86_64-unknown-linux-gnu"  # Replace with your platform

# Download artifact
curl -LO "https://github.com/eqtylab/cupcake/releases/download/${VERSION}/cupcake-${VERSION}-${PLATFORM}.tar.gz"

# Download provenance
curl -LO "https://github.com/eqtylab/cupcake/releases/download/${VERSION}/multiple.intoto.jsonl"
```

**Available platforms**:
- `x86_64-unknown-linux-gnu` - Linux x64 (glibc)
- `x86_64-unknown-linux-musl` - Linux x64 (musl, static)
- `aarch64-unknown-linux-gnu` - Linux ARM64
- `x86_64-apple-darwin` - macOS Intel
- `aarch64-apple-darwin` - macOS Apple Silicon
- `x86_64-pc-windows-msvc` - Windows x64 (use `.zip` extension)

### Step 3: Verify the Artifact

```bash
slsa-verifier verify-artifact \
  --provenance-path multiple.intoto.jsonl \
  --source-uri github.com/eqtylab/cupcake \
  --source-tag "${VERSION}" \
  "cupcake-${VERSION}-${PLATFORM}.tar.gz"
```

**Expected output**:
```
Verified build using builder "https://github.com/slsa-framework/slsa-github-generator/.github/workflows/generator_generic_slsa3.yml@refs/tags/v2.1.0" at commit <commit-sha>
Verifying artifact cupcake-<version>-<platform>.tar.gz: PASSED

PASSED: SLSA verification passed
```

## Inspecting Provenance

You can inspect the provenance to see what's included:

### View all artifacts in provenance:
```bash
jq -r '.dsseEnvelope.payload' multiple.intoto.jsonl | base64 -d | jq '.subject[].name'
```

### View build details:
```bash
jq -r '.dsseEnvelope.payload' multiple.intoto.jsonl | base64 -d | jq '.predicate.buildDefinition'
```

### View source information:
```bash
jq -r '.dsseEnvelope.payload' multiple.intoto.jsonl | base64 -d | jq '.predicate.runDetails'
```

## What Does This Prove?

When `slsa-verifier` returns "PASSED", it cryptographically proves:

✅ **Authentic source**: Binary was built from the exact commit in the official eqtylab/cupcake repository
✅ **Unmodified**: File hash matches what was recorded at build time
✅ **Official build**: Built by GitHub Actions using the official release workflow
✅ **Non-forgeable**: Signed by GitHub's OIDC infrastructure (maintainers cannot fake this)

## Threat Model

SLSA Level 3 protects against:

- ❌ **Compromised build environment**: Attacker cannot forge provenance (no access to signing keys)
- ❌ **Tampered artifacts**: Hash mismatch detected immediately
- ❌ **Unauthorized builds**: Provenance includes source commit SHA, detects builds from wrong repo/branch
- ❌ **Compromised maintainer account**: Cannot create fake provenance without GitHub's signing infrastructure

## Technical Details

Cupcake's SLSA implementation:
- **Generator**: [`slsa-github-generator@v2.1.0`](https://github.com/slsa-framework/slsa-github-generator)
- **Workflow**: [`.github/workflows/release.yml`](../../.github/workflows/release.yml)
- **Format**: [in-toto attestation](https://github.com/in-toto/attestation) with DSSE envelope
- **Signing**: GitHub OIDC tokens + Sigstore transparency log

## Resources

- [SLSA Framework](https://slsa.dev/) - Official specification
- [slsa-verifier](https://github.com/slsa-framework/slsa-verifier) - Verification tool
- [Cupcake Release Workflow](../../.github/workflows/release.yml) - Our implementation
- [GitHub SLSA Support](https://github.blog/2022-04-07-slsa-3-compliance-with-github-actions/) - How GitHub achieves L3
