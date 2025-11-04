# SLSA Level 3 for Rust: Implementation Guide

Rust projects achieve SLSA Build Level 3 using the generic slsa-github-generator workflow, not a dedicated Rust builder. As of November 2025, **slsa-github-generator v2.1.0** is the current stable release, while GitHub's native Artifact Attestations (GA June 2024) offers a simpler alternative reaching Level 2 by default and Level 3 with reusable workflows.

**Critical update**: All users on v1.9.0 or earlier must upgrade to v1.10.0+ to avoid TUF mirror errors that break builds. Version 2.0.0 introduced breaking changes requiring actions/upload-artifact@v4 and Node 20 compatibility. For Rust specifically, there's no native cargo integration yet—developers must manually generate SHA256 hashes of built binaries and pass them to the generic provenance generator.

## Two paths to SLSA Level 3 in 2024-2025

**slsa-github-generator v2.1.0** provides the mature, battle-tested approach with extensive customization options and language-specific builders for Go, Node.js, Maven, and Gradle. Rust projects use the **generic generator** (`generator_generic_slsa3.yml`) which works for any artifact type. This approach separates the build job from provenance generation—your workflow builds artifacts, calculates their SHA256 hashes, then invokes the reusable SLSA workflow to generate signed provenance. The isolated provenance generation job has no access to your build environment or secrets, satisfying Level 3's requirement that user code cannot tamper with signing material.

**GitHub Artifact Attestations** takes a different approach, integrating provenance generation directly into your build workflow with a simple four-line action call. This achieves Level 2 by default since the build and attestation occur in the same job. To reach Level 3, you must refactor your build into a **reusable workflow**—the isolation boundary between the calling workflow and the reusable workflow provides the required separation. GitHub handles signing through Sigstore's OIDC-based infrastructure automatically.

For new Rust projects in 2025, GitHub Artifact Attestations offers the simplest path. For projects requiring matrix builds across multiple platforms, cross-compilation workflows, or migration from existing slsa-github-generator implementations, continuing with the generic generator remains the most practical choice.

## SLSA Level 3 requirements explained

Level 3 demands **build isolation** ensuring each build runs in an ephemeral environment with no state persistence between runs, **secret material protection** preventing user-defined build steps from accessing provenance signing keys, and **complete provenance generation** describing the build process with cryptographically signed attestations resistant to tampering. GitHub Actions hosted runners provide the ephemeral isolation. The key differentiator is keeping signing material away from user code—achieved by running provenance generation in a separate job with isolated permissions.

The provenance itself follows the in-toto attestation format, capturing the source repository URI, commit SHA, workflow details, build inputs, and SHA256 digests of all output artifacts. This provenance is signed using Sigstore's keyless signing via GitHub's OIDC tokens, then posted to the public Rekor transparency log for verifiable audit trails. Users can verify any artifact by checking that its hash matches the provenance, the provenance signature is valid, and the provenance claims the artifact was built from your specific repository and commit.

## Rust workflow implementation with generic generator

The complete workflow pattern for Rust projects spans three jobs: build, hash combination (for matrix builds), and provenance generation. **The build job must output base64-encoded SHA256 hashes** of all artifacts—this is the most critical integration point between your build and SLSA provenance.

In the build job, install Rust with dtolnay/rust-toolchain, build your release binary with cargo build --release, then generate the subject hash. The hash format is crucial: run `sha256sum target/release/my-binary` to get output like `abc123... my-binary`, then base64 encode it with `base64 -w0`. Set this as a job output. Upload your built binary using actions/upload-artifact@v4. The provenance job runs separately with minimal permissions—only `actions: read`, `id-token: write`, and `contents: write` if uploading to releases. It calls the reusable workflow at `slsa-framework/slsa-github-generator/.github/workflows/generator_generic_slsa3.yml@v2.1.0`, passing the base64-encoded hashes.

**Critical requirement**: Reference the generator by full semantic version tag (@v2.1.0), never @v2 or @main. The slsa-verifier tool requires exact semantic versions for verification. This contradicts GitHub's typical best practice of pinning to commit SHAs but is necessary due to Actions limitations.

For Rust specifically, artifacts live at `target/release/binary-name` for native builds or `target/<TARGET>/release/binary-name` for cross-compilation. Windows builds need `.exe` extensions. When calculating hashes, account for these path variations. Many Rust workflows use matrix strategies to build for multiple targets—Linux x86/ARM, macOS Intel/Apple Silicon, Windows—each producing binaries at different paths with different names.

## Matrix builds and multi-artifact provenance

Matrix builds pose a specific challenge: each matrix iteration runs independently, generating separate artifacts and hashes. **Two approaches handle this**: generate one provenance file covering all artifacts, or generate separate provenance per artifact.

The **single provenance approach** requires a combining step. Each matrix job calculates its artifact hash and uploads it as a separate artifact. After all matrix jobs complete, a combine-hashes job downloads all hash artifacts, concatenates them, and outputs the combined base64-encoded list. The provenance job then generates one attestation covering all artifacts. This is simpler for verification since users download one provenance file that covers all platforms.

Example matrix configuration for Rust cross-platform builds:

```yaml
build:
  strategy:
    matrix:
      include:
        - os: ubuntu-latest
          target: x86_64-unknown-linux-gnu
          name: linux-amd64
        - os: ubuntu-latest
          target: aarch64-unknown-linux-gnu
          name: linux-arm64
          use-cross: true
        - os: windows-latest
          target: x86_64-pc-windows-msvc
          name: windows-amd64
        - os: macos-latest
          target: aarch64-apple-darwin
          name: macos-arm64
```

Each matrix job installs the target with `rustup target add`, builds with `cargo build --release --target`, and generates platform-specific hashes. For exotic targets like ARM on x86 hosts, set `use-cross: true` to use the cross tool, which leverages Docker for cross-compilation. Native targets compile faster and should be preferred when possible—macOS can cross-compile between Intel and Apple Silicon natively, for instance.

The **multiple provenance approach** runs the provenance job in a matrix itself, generating separate provenance files per artifact. This increases complexity for users who must verify multiple provenance files, but provides clearer separation if artifacts have different build configurations or requirements.

For **multi-artifact scenarios** where a single repository produces multiple binaries (like a workspace with multiple packages), generate all artifacts in one build step, calculate all hashes together with `sha256sum target/release/*`, and pass the combined hash list to the provenance generator. The provenance will list all artifacts as subjects.

## Repository configuration and permissions

**OIDC token permission is mandatory**. The provenance generation job must have `id-token: write` permission explicitly set at the job level. This allows requesting OIDC JWT tokens from GitHub's OIDC provider for Sigstore signing. This permission does not grant write access to code or resources—it only permits obtaining authentication tokens for keyless signing.

The minimal permission set for provenance generation:

```yaml
permissions:
  actions: read # Detect GitHub Actions environment
  id-token: write # Sign provenance with OIDC
  contents: write # Upload to releases (optional)
```

For private repositories, **critical security consideration**: you must explicitly set `private-repository: true` in the workflow inputs. Without this flag, the workflow intentionally fails to prevent accidentally leaking repository information to public Rekor transparency logs. Private repository names, commit SHAs, and workflow details become permanently public once posted to Rekor. Organizations should carefully evaluate whether this information disclosure is acceptable.

**Trigger event restrictions** affect SLSA workflows. The `pull_request` trigger from forked repositories cannot access `id-token: write` due to GitHub security restrictions preventing malicious forks from obtaining signing tokens. Test SLSA workflows using `push`, `release`, `workflow_dispatch`, or `pull_request` events from the same repository (though attestations won't be signed in PR context). For release workflows, trigger on tag pushes or release creation events.

**GitHub-hosted runners provide the ephemeral isolation** required for Level 3. Each job runs in a fresh VM destroyed after completion, preventing state persistence or cross-contamination between builds. Self-hosted runners can be used but require careful configuration to ensure equivalent isolation guarantees.

## Verification procedures and tooling

The **slsa-verifier CLI** is the standard verification tool. Install it via `go install github.com/slsa-framework/slsa-verifier/v2/cli/slsa-verifier@latest` or download prebuilt binaries from GitHub releases. Current version is v2.7.1. For Homebrew users, `brew install slsa-verifier` works on macOS.

**Basic verification command** for Rust binaries:

```bash
slsa-verifier verify-artifact \
  --artifact-path ./my-app-linux-amd64 \
  --provenance-path ./provenance.intoto.jsonl \
  --source-uri github.com/yourorg/yourrepo \
  --source-tag v1.0.0
```

The verifier checks that the artifact's SHA256 hash matches a subject in the provenance, the provenance signature is cryptographically valid and signed by GitHub's OIDC issuer, and the provenance claims the artifact was built from the specified source repository and tag. If all checks pass, it prints "PASSED: Verified SLSA provenance". Any verification failure indicates potential tampering or supply chain compromise.

For **matrix builds with multiple artifacts**, verify each binary against the same provenance file. The provenance contains multiple subjects, and the verifier automatically matches the artifact hash to the correct subject entry. Users downloading your linux-arm64 binary verify it with `--artifact-path ./my-app-linux-arm64` using the same shared provenance.

**Source URI and tag matching** provides additional security. The `--source-uri` flag ensures the artifact was built from your official repository, not a fork or malicious copy. The `--source-tag` flag verifies the exact release version. For branch builds, use `--source-branch main` instead. The `--source-versioned-tag` option enables semantic version matching, allowing verification of "v1.\*" against any v1.x.x tag.

**Container image verification** requires the immutable digest rather than tag. Tags are mutable and vulnerable to TOCTOU (time-of-check to time-of-use) attacks where an attacker replaces the image between verification and use:

```bash
# Get immutable reference
IMAGE="ghcr.io/org/image:tag"
IMAGE="${IMAGE}@$(crane digest ${IMAGE})"

# Verify with digest
slsa-verifier verify-image "${IMAGE}" \
  --source-uri github.com/org/repo \
  --source-tag v1.0.0
```

For **GitHub Artifact Attestations**, use the GitHub CLI instead: `gh attestation verify my-app --owner yourorg`. The attestation is stored alongside the artifact in GitHub's infrastructure and verified automatically. With reusable workflows achieving Level 3, add `--signer-repo` and `--signer-workflow` flags specifying the reusable workflow that performed the isolated build.

## Version updates and breaking changes

**The TUF mirror crisis** affected all v1.9.0 and earlier versions in early 2024. Users experienced "error updating to TUF remote mirror: tuf: invalid key" causing complete build failures. This stemmed from Sigstore's cosign TUF roots rotation. Version 1.10.0 (March 2024) fixed this permanently. **Immediate action required**: any workflow using v1.9.0 or earlier must upgrade. Temporary workaround is setting `compile-generator: true`, which compiles the generator binary from source, bypassing the pre-built binary verification that depends on the broken TUF state. This adds approximately 2 minutes to build time.

**Version 2.0.0 (January 2024)** introduced Node 20 migration, upgrading from actions/upload-artifact@v3 to @v4 and actions/download-artifact@v3 to @v4. These are not backwards compatible—workflows cannot mix v3 and v4 of these actions. When upgrading to slsa-github-generator v2.0.0+, update all upload-artifact and download-artifact references in your workflows to @v4. The artifact actions team made this breaking change due to Node 16 deprecation across the Actions ecosystem.

**Version 2.1.0 (February 2024)** added `base64-subjects-as-file` input for large subject lists exceeding shell argument limits, improved error messages, and stability fixes. This is the current recommended version for all new implementations.

**GitHub Artifact Attestations** reached General Availability in June 2024 after extensive beta testing. December 2024 guidance positioned it as the recommended approach for new projects. The attestations feature integrates tightly with GitHub's infrastructure—attestations are stored in GitHub's artifact registry alongside artifacts, searchable via GitHub's UI, and verifiable with built-in tooling. This reduces external dependencies compared to slsa-github-generator's reliance on Rekor, Sigstore, and the verifier CLI.

Migration from v1.x to v2.x requires updating the generator reference, updating all artifact actions to @v4, and retesting verification workflows. No changes to permissions or hash generation logic are necessary. The provenance format remains compatible, allowing gradual migration.

## Common pitfalls and solutions

**Builder reference format errors** are the most frequent mistake. Using `@v2` or `@v2.1` instead of `@v2.1.0` causes verification failures with cryptic "slsa-verifier cannot verify the ref" errors. The verifier strictly requires full semantic versions. GitHub Actions allows shortened version references (v2, v2.1) that automatically resolve to the latest matching version, but this prevents verification since the exact version is unknowable. Always use full three-part version tags.

**OIDC token permission errors** manifest as "Ensure GITHUB_TOKEN has permission 'id-token: write'" or "Unable to get ACTIONS_ID_TOKEN_REQUEST_URL env variable". This means permissions are not set correctly. Permissions can be set at workflow level or job level—job-level is preferred for least privilege. For reusable workflows, the **caller must grant permissions**:

```yaml
jobs:
  call-slsa:
    permissions:
      id-token: write
      contents: read
    uses: org/repo/.github/workflows/reusable.yml@v1
```

GitHub's permission inheritance is complex. Default workflow permissions (configured in repository settings) don't automatically flow to reusable workflow calls—they must be explicitly granted. Pull requests from forks never receive id-token write permission for security reasons.

**Private repository transparency log errors** occur when private repositories attempt SLSA workflows without setting `private-repository: true`. The workflow intentionally fails with "private repositories will generate an error in order to prevent leaking repository name information". This is a safety mechanism. If you acknowledge that repository information will be posted to the public Rekor log, add the flag. There is no private transparency log option currently—tracked in issue #372.

**Artifact path confusion** trips up Rust workflows using cross-compilation. Native builds output to `target/release/binary`, but cross-compiled builds output to `target/x86_64-unknown-linux-musl/release/binary`. Hash generation must account for the correct path. Matrix strategies help by templating the target into the path: `target/${{ matrix.target }}/release/binary`. For Windows, remember the .exe extension—use conditional logic to append it only on Windows runners.

**Rekor sharding issues** occasionally cause "Generate Builder" step failures with "SLSA verification failed: could not find a matching valid signature entry". This stems from Rekor transparency log infrastructure changes affecting verification of pre-built binaries. Enable `compile-builder: true` as a workaround, which compiles the builder from source instead of downloading pre-built binaries. This bypasses verification dependent on Rekor state but increases build time by ~2 minutes.

**Hash format mistakes** break provenance generation. The generator expects base64-encoded output in the format "hash filename" per line (matching sha256sum output), then base64 encoded with `base64 -w0` (the -w0 flag prevents line wrapping). Common errors include forgetting base64 encoding, using incorrect hash algorithms (must be SHA256), or including extra whitespace. Test hash generation locally before pushing to CI.

## Real-world Rust implementations

**No production Rust projects have dedicated SLSA builders yet**. The Rust community is actively discussing native cargo integration (rust-lang/cargo#12661) but as of November 2025, this remains in proposal stages. The discussion centers on whether to create a dedicated Rust builder using the BYOB (Bring Your Own Builder) framework or integrate provenance generation directly into cargo. The latter approach would publish provenance alongside crate uploads to crates.io, similar to how npm handles provenance in the Node.js ecosystem.

**slsa-example by aquint-zama** demonstrates the generic generator pattern for Rust. This example crate publishes to crates.io with SLSA provenance, showing end-to-end workflow from build to verification. It uses the standard pattern of cargo build, hash generation, and provenance generation described earlier.

**Popular projects using generic generator** include urllib3, ko, jib, grpc-gateway, flatbuffers, Kyverno (containers), and Flux CD. While these aren't Rust projects, they demonstrate the generic generator's maturity and production-readiness across diverse ecosystems. The workflow patterns are directly applicable to Rust.

The **SLSA blog specifically mentions Rust** in discussing supply chain attacks, citing cases where attackers compromised Rust crate maintainer accounts through expired domain takeovers. SLSA provenance prevents these attacks by including `actor_id` and `repository_id` in attestations—even if an attacker gains account access, they cannot forge provenance from the legitimate repository without access to GitHub's OIDC signing infrastructure.

## Container-based builder for Docker workflows

For Rust projects shipping Docker containers, the **container-based SLSA 3 builder** (`generator_container_slsa3.yml`) offers an alternative approach announced June 2023. This builder includes the entire build process within the provenance, providing stronger verification guarantees than the generic generator.

The workflow builds a Rust Docker image using standard Docker actions, captures the image digest, then calls the container generator with the image reference and digest. The provenance includes the builder image URI (e.g., `rust@sha256:...`), build commands (`cargo build --release`), and output artifact paths. This creates a complete audit trail from source to container image.

Container verification uses `slsa-verifier verify-image` with the image digest and source URI. For container registries like ghcr.io, set `GH_TOKEN` environment variable for authentication. Always use immutable digest references (@sha256:...) rather than tags to prevent TOCTOU attacks.

## Implementation checklist for Rust projects

Start by choosing your approach—GitHub Artifact Attestations for simplicity, or slsa-github-generator for maximum flexibility and maturity. For new projects, attestations offer the fastest path to Level 3 with reusable workflows. For projects already using slsa-github-generator or requiring complex matrix builds, continue with the generic generator at v2.1.0.

Set up your build job with Rust toolchain installation, cargo build commands, and hash generation. For matrix builds, define platform targets and configure cross-compilation tooling. Use the `cross` tool for exotic targets requiring emulation, but prefer native cross-compilation where possible (macOS ARM/Intel, Linux x86/ARM with appropriate toolchains).

Configure the provenance job with minimal permissions—only what's strictly necessary for signing and uploading. Reference the generator workflow by full semantic version tag (@v2.1.0). For private repositories, carefully consider whether posting to public Rekor logs is acceptable and set the flag explicitly.

Test verification before releasing to users. Download generated provenance, run slsa-verifier locally against your built artifacts, and ensure verification succeeds. Document verification procedures in your README so users understand how to verify downloads. Include specific commands with your repository URI and expected tags.

Set up dependency tracking for slsa-github-generator and slsa-verifier updates using Renovate or Dependabot. Monitor the slsa-framework GitHub organization for security advisories and breaking changes. Join the OpenSSF Slack #slsa-tooling channel for community support and announcements.

## Future developments and cargo integration

The Rust ecosystem is moving toward **native cargo SLSA support**. Discussions in rust-lang/cargo#12661 explore automatic provenance generation during `cargo publish`, publishing provenance alongside crate uploads to crates.io, and providing verification tooling integrated into cargo itself. This would eliminate the need for separate CI workflow configuration and make SLSA provenance ubiquitous across the entire Rust ecosystem.

**The BYOB framework** offers a path to create dedicated Rust builders today. BYOB lets you wrap arbitrary build processes in SLSA-compliant isolation while maintaining Level 3 guarantees. A community-contributed Rust builder using BYOB could provide cargo-native integration, better error messages, and Rust-specific optimizations compared to the generic generator.

**GitHub's attestations feature continues evolving**, with potential future support for private transparency logs, enhanced policy enforcement, and tighter integration with GitHub's security features like code scanning and secret scanning. This could enable automated security policies rejecting deployments of artifacts without valid SLSA provenance.

For now, **the generic generator remains the production-ready path** for Rust projects requiring SLSA Level 3. It provides strong security guarantees, active maintenance, and clear upgrade paths as the ecosystem matures. As native cargo integration develops, migration should be straightforward since the underlying attestation formats and verification procedures remain consistent across implementations.

## Conclusion

Implementing SLSA Level 3 for Rust projects in GitHub Actions requires using the generic slsa-github-generator workflow at version 2.1.0 or GitHub's Artifact Attestations with reusable workflows. Both approaches achieve Level 3 compliance through isolated provenance generation with protected signing material. The key technical requirements are calculating SHA256 hashes of built artifacts, passing them to the provenance generator, configuring OIDC token permissions, and verifying attestations with slsa-verifier.

Matrix builds across multiple platforms work seamlessly by combining hashes from all matrix iterations into a single provenance file. Critical pitfalls include using incorrect version tag formats, missing OIDC permissions, and private repository configuration. Version 1.10.0+ is mandatory to avoid TUF mirror failures, while v2.0.0+ requires actions/upload-artifact@v4 compatibility.

The Rust community is actively working toward native cargo integration, but the generic generator provides a mature, production-ready solution today. Combined with proper verification procedures and documentation, it delivers strong supply chain security protecting against compromise from build to distribution.
