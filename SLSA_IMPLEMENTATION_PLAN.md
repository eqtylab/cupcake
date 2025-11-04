# SLSA Level 3 Implementation Plan

**Status**: Ready for implementation
**Target**: Add SLSA Build Level 3 provenance to existing release workflow
**Approach**: Artifact-based matrix aggregation pattern
**Estimated changes**: ~60 lines added to release.yml

## Objectives

1. Generate signed SLSA provenance for all 6 platform builds
2. Upload `multiple.intoto.jsonl` to GitHub releases
3. Enable users to verify artifacts with `slsa-verifier`
4. Achieve SLSA Build Level 3 compliance
5. Maintain existing release workflow behavior

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Existing Workflow (NO CHANGES)                             │
├─────────────────────────────────────────────────────────────┤
│  create-release → build (matrix 6x) → checksums → finalize  │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  NEW: SLSA Provenance Generation                            │
├─────────────────────────────────────────────────────────────┤
│  build → [artifacts: hash-*] → combine-hashes → provenance  │
│                                                               │
│  VM #1-6: Generate hashes                                    │
│  VM #7:   Aggregate all hashes                               │
│  VM #8:   Generate provenance (isolated, signed)             │
└─────────────────────────────────────────────────────────────┘
```

## Technical Design

### 1. Hash Generation (Added to Build Job)

**Location**: After checksum generation (line 218 Unix / 265 Windows)

**Implementation**:
- Reuse existing SHA256 checksums (no duplicate calculation)
- Write hash to file: `${{ matrix.name }}.sha256`
- Upload as artifact with unique name: `hash-${{ matrix.name }}`
- Use `actions/upload-artifact@v4` for pattern support

**Why artifacts not outputs**: Matrix jobs cannot aggregate outputs (last-write-wins). Research documented in `MATRIX_OUTPUTS.md`.

### 2. Hash Aggregation (New Job)

**Job**: `combine-hashes`
**Purpose**: Collect all 6 platform hashes into single base64-encoded blob
**Dependencies**: `needs: [build]`

**Implementation**:
- Download all hash artifacts with pattern `hash-*`
- Merge into single directory
- Validate count == 6 (fail if mismatch)
- Base64 encode combined hashes
- Output via `$GITHUB_OUTPUT` (trusted channel)

**Security**: Only hashes cross VM boundaries, not artifacts themselves.

### 3. Provenance Generation (New Job)

**Job**: `provenance`
**Type**: Reusable workflow from slsa-framework
**Dependencies**: `needs: [combine-hashes, create-release]`

**Configuration**:
```yaml
uses: slsa-framework/slsa-github-generator/.github/workflows/generator_generic_slsa3.yml@v2.1.0
with:
  base64-subjects: ${{ needs.combine-hashes.outputs.hashes }}
  upload-assets: true
  upload-tag-name: ${{ needs.create-release.outputs.version }}
```

**Permissions** (isolated from build):
- `actions: read` - Detect GitHub Actions environment
- `id-token: write` - Sign provenance with OIDC
- `contents: write` - Upload to release

**Output**: `multiple.intoto.jsonl` uploaded to GitHub release

### 4. Dependency Updates

**Jobs to update**:
- `checksums`: Add `provenance` to needs
- `finalize`: Add `provenance` to needs

**Purpose**: Ensure provenance completes before release is published

## SLSA Level 3 Requirements

| Requirement | Implementation | VM Isolation |
|-------------|----------------|--------------|
| **Non-forgeable provenance** | slsa-github-generator@v2.1.0 uses GitHub OIDC tokens | Generator runs on isolated VM with no access to build secrets |
| **Build isolation** | Build job has NO `id-token` permission | Build VMs #1-6 cannot access signing material |
| **Ephemeral environment** | GitHub-hosted runners | Fresh VMs destroyed after each job |

## Implementation Checklist

### Phase 1: Workflow Modification
- [ ] Add hash generation step to build job (after line 218/265)
- [ ] Add artifact upload step to build job
- [ ] Create combine-hashes job
- [ ] Create provenance job
- [ ] Update checksums job dependencies
- [ ] Update finalize job dependencies

### Phase 2: Testing
- [ ] Create test tag: `v0.2.0-slsa-test`
- [ ] Verify workflow runs without errors
- [ ] Verify 6 hash artifacts uploaded
- [ ] Verify `multiple.intoto.jsonl` in release assets
- [ ] Download provenance and inspect with `cat multiple.intoto.jsonl | jq`
- [ ] Install slsa-verifier: `brew install slsa-verifier`
- [ ] Verify each platform artifact against provenance

### Phase 3: Documentation
- [ ] Add SLSA verification section to README.md
- [ ] Document verification commands for users
- [ ] Add SLSA badge to README (optional)
- [ ] Update release notes template with verification instructions

### Phase 4: Production Release
- [ ] Delete test tag and release
- [ ] Create real release tag (e.g., `v0.2.0`)
- [ ] Verify production provenance
- [ ] Announce SLSA Level 3 compliance

## Verification Commands

### Install slsa-verifier

```bash
# macOS
brew install slsa-verifier

# Linux/WSL
go install github.com/slsa-framework/slsa-verifier/v2/cli/slsa-verifier@latest

# Verify installation
slsa-verifier version
```

### Verify artifact (example for linux-x64)

```bash
# Download artifact and provenance
VERSION="v0.2.0"
PLATFORM="x86_64-unknown-linux-gnu"

curl -LO https://github.com/eqtylab/cupcake/releases/download/${VERSION}/cupcake-${VERSION}-${PLATFORM}.tar.gz
curl -LO https://github.com/eqtylab/cupcake/releases/download/${VERSION}/multiple.intoto.jsonl

# Verify
slsa-verifier verify-artifact \
  --provenance-path multiple.intoto.jsonl \
  --source-uri github.com/eqtylab/cupcake \
  --source-tag ${VERSION} \
  cupcake-${VERSION}-${PLATFORM}.tar.gz

# Expected output: "PASSED: Verified SLSA provenance"
```

### Inspect provenance contents

```bash
cat multiple.intoto.jsonl | jq '.payload | @base64d | fromjson | .predicate.materials'
```

Shows source repo, commit SHA, and build inputs.

## Rollback Plan

If issues discovered after deployment:

1. **Workflow still works without provenance** - other jobs unchanged
2. **Remove provenance job** from workflow to disable
3. **Delete `multiple.intoto.jsonl`** from release if needed
4. **No impact to users** - provenance is optional verification

## Future Enhancements

1. **Add SLSA badge** to README: `[![SLSA 3](https://slsa.dev/images/gh-badge-level3.svg)](https://slsa.dev)`
2. **Automated verification** in install scripts
3. **Renovate rule** for slsa-github-generator updates
4. **Policy enforcement** requiring SLSA verification before deployment

## References

- SLSA Framework: https://slsa.dev/
- slsa-github-generator: https://github.com/slsa-framework/slsa-github-generator
- Matrix outputs research: `MATRIX_OUTPUTS.md`
- Implementation approach: `SLSA_APPROACH.md`
- Level 3 requirements: `SLSA_L3.md`
- Technical details: `SLSA_KEY_INFO.md`

## Success Criteria

✅ All 6 platform artifacts listed in `multiple.intoto.jsonl`
✅ Provenance signature validates against GitHub OIDC issuer
✅ `slsa-verifier` returns "PASSED" for all platforms
✅ Provenance includes correct source repo and commit SHA
✅ Release workflow completes without errors
✅ No impact to existing release artifacts or checksums
