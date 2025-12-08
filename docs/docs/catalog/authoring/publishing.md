---
title: "Publishing"
description: "How to publish your rulebook to the catalog"
---

# Publishing

This guide explains how to publish your rulebook to the official Cupcake Catalog.

## Before Publishing

### 1. Validate Your Rulebook

```bash
cupcake catalog lint ./my-rulebook
```

Fix any errors or warnings before proceeding.

### 2. Test Locally

Install your rulebook locally to verify it works:

```bash
# Package it
cupcake catalog package ./my-rulebook --output ./dist

# Manually extract to test location
mkdir -p .cupcake/catalog/my-rulebook
tar -xzf ./dist/my-rulebook-1.0.0.tar.gz -C .cupcake/catalog/

# Create a test event
cat > test-event.json << 'EOF'
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": { "command": "echo test" },
  "session_id": "test",
  "cwd": "/tmp",
  "transcript_path": "/tmp/transcript.md"
}
EOF

# Test with the sample input
cupcake eval --harness claude < test-event.json
```

### 3. Write Documentation

Every rulebook should include:

- **README.md** - What it does, how to use it, configuration options
- **CHANGELOG.md** - Version history and changes

### 4. Package for Distribution

```bash
cupcake catalog package ./my-rulebook --output ./dist
```

This creates:

- `my-rulebook-1.0.0.tar.gz` - The distributable tarball
- SHA256 digest displayed in output

## Publishing Process

### Option 1: Pull Request to Official Catalog

1. **Fork** the [cupcake-catalog](https://github.com/eqtylab/cupcake-catalog) repository

2. **Add your rulebook** to the `rulebooks/` directory:

   ```
   rulebooks/
   └── my-rulebook/
       ├── manifest.yaml
       ├── README.md
       ├── CHANGELOG.md
       └── policies/
           └── ...
   ```

3. **Create a pull request** with:
   - Description of what the rulebook does
   - Use cases it addresses
   - Any security considerations

4. **CI validation** runs automatically:
   - Manifest schema validation
   - Namespace convention checks
   - Policy syntax verification

5. **Review and merge** - Maintainers will review your submission

6. **Release** - Once merged, a release is created and the index is updated

### Option 2: Host Your Own Registry

For private or organization-specific rulebooks:

1. **Create an index.yaml** for your rulebooks:

   ```yaml
   apiVersion: cupcake.dev/v1
   kind: CatalogIndex
   generated: "2025-12-04T00:00:00Z"
   entries:
     my-rulebook:
       - name: my-rulebook
         version: "1.0.0"
         description: My custom rulebook
         harnesses: [claude, cursor]
         keywords: [custom]
         digest: "sha256:abc123..."
         urls:
           - https://my-server.com/releases/my-rulebook-1.0.0.tar.gz
   ```

2. **Host the files** on a web server:
   - `index.yaml` at the registry root
   - Tarball at the URL specified in `urls`

3. **Add the registry** to Cupcake:

   ```bash
   cupcake catalog repo add mycompany https://my-server.com/index.yaml
   ```

## Index Generation

The official catalog uses a script to generate `index.yaml`:

```bash
python scripts/generate-index.py
```

This scans `rulebooks/` and builds the index with:

- All rulebook metadata from manifests
- Version history
- Download URLs pointing to GitHub releases

## Versioning Guidelines

Follow [Semantic Versioning](https://semver.org/):

- **MAJOR** (1.0.0 → 2.0.0) - Breaking changes
- **MINOR** (1.0.0 → 1.1.0) - New features, backward compatible
- **PATCH** (1.0.0 → 1.0.1) - Bug fixes, backward compatible

### When to Bump Versions

| Change                     | Version Bump   |
| -------------------------- | -------------- |
| New policy added           | MINOR          |
| Policy logic changed       | MINOR or MAJOR |
| Metadata update only       | PATCH          |
| Breaking namespace change  | MAJOR          |
| Bug fix in existing policy | PATCH          |

## Release Checklist

- [ ] All tests pass locally
- [ ] `cupcake catalog lint` reports no errors
- [ ] README.md is up to date
- [ ] CHANGELOG.md updated with new version
- [ ] Version bumped in manifest.yaml
- [ ] Pull request created with description

## After Publishing

Once your rulebook is in the catalog:

```bash
# Users can find it
cupcake catalog search my-rulebook

# And install it
cupcake catalog install my-rulebook
```

Monitor issues and feedback on the GitHub repository to improve your rulebook over time.
