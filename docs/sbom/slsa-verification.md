# SLSA Level 3 Verification

Cupcake releases include SLSA Build Level 3 provenance. This provides cryptographic proof of build integrity and source authenticity, meeting enterprise supply chain security requirements that are difficult to achieve through other means.

SLSA Level 3 guarantees: non-forgeable provenance (signed by GitHub OIDC, not maintainer-controlled), isolated build environments, and ephemeral infrastructure. Each release includes a `multiple.intoto.jsonl` attestation covering all platform binaries.

## Quick Verification

```bash
curl -fsSL https://raw.githubusercontent.com/eqtylab/cupcake/main/scripts/verify-release.sh | bash
```

Or download and run locally: [`scripts/verify-release.sh`](../../scripts/verify-release.sh)

## Manual Verification

Install slsa-verifier:
```bash
# macOS
brew install slsa-verifier

# Linux
LATEST=$(curl -s https://api.github.com/repos/slsa-framework/slsa-verifier/releases/latest | grep tag_name | cut -d '"' -f 4)
curl -Lo slsa-verifier "https://github.com/slsa-framework/slsa-verifier/releases/download/${LATEST}/slsa-verifier-linux-amd64"
chmod +x slsa-verifier
sudo mv slsa-verifier /usr/local/bin/
```

Download release assets:

```bash
VERSION="v0.1.0"  # Replace with desired version
PLATFORM="x86_64-unknown-linux-gnu"  # Replace with your platform

# Download artifact
curl -LO "https://github.com/eqtylab/cupcake/releases/download/${VERSION}/cupcake-${VERSION}-${PLATFORM}.tar.gz"

# Download provenance
curl -LO "https://github.com/eqtylab/cupcake/releases/download/${VERSION}/multiple.intoto.jsonl"
```

Platforms: `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc` (`.zip`)

Verify artifact:

```bash
slsa-verifier verify-artifact \
  --provenance-path multiple.intoto.jsonl \
  --source-uri github.com/eqtylab/cupcake \
  --source-tag "${VERSION}" \
  "cupcake-${VERSION}-${PLATFORM}.tar.gz"
```

Expected output: `PASSED: SLSA verification passed`

## Inspect Provenance

View all artifacts:
```bash
jq -r '.dsseEnvelope.payload' multiple.intoto.jsonl | base64 -d | jq '.subject[].name'
```

View build details:
```bash
jq -r '.dsseEnvelope.payload' multiple.intoto.jsonl | base64 -d | jq '.predicate.buildDefinition'
```

View source information:
```bash
jq -r '.dsseEnvelope.payload' multiple.intoto.jsonl | base64 -d | jq '.predicate.runDetails'
```

## What Verification Proves

Successful verification cryptographically confirms: binary built from stated commit in official repository, file hash matches build-time recording, executed via official GitHub Actions workflow, signed by GitHub OIDC (not maintainer-controlled).

SLSA Level 3 mitigates: compromised build environments, tampered artifacts, unauthorized builds, and compromised maintainer accounts.

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
