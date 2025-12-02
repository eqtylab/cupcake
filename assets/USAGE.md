# Cupcake CLI Usage

Visual demonstrations of the Cupcake command-line interface.

## Initialize a Project

Set up Cupcake in your project with harness-specific configuration:

```bash
cupcake init --harness claude
```

![Cupcake Init](./output/gifs/cupcake-init.gif)

Creates the `.cupcake/` directory structure with policies, signals, and configuration.

---

## View Help

See all available commands and options:

```bash
cupcake --help
```

![Cupcake Help](./output/gifs/cupcake-help.gif)

---

## Inspect Policies

View loaded policies, their routing metadata, and enabled builtins:

```bash
cupcake inspect
cupcake inspect --table  # Compact table view
```

![Cupcake Inspect](./output/gifs/cupcake-inspect.gif)

---

## Verify Configuration

Validate your policies and configuration:

```bash
cupcake verify --harness claude
```

![Cupcake Verify](./output/gifs/cupcake-verify.gif)

---

## Trust Management

Manage script trust and integrity verification:

```bash
cupcake trust init      # Initialize trust manifest
cupcake trust list      # List trusted scripts
cupcake trust verify    # Verify against manifest
```

![Cupcake Trust](./output/gifs/cupcake-trust.gif)

---

## Supported Harnesses

| Harness    | Agent       | Command                           |
| ---------- | ----------- | --------------------------------- |
| `claude`   | Claude Code | `cupcake init --harness claude`   |
| `cursor`   | Cursor      | `cupcake init --harness cursor`   |
| `factory`  | Factory AI  | `cupcake init --harness factory`  |
| `opencode` | OpenCode    | `cupcake init --harness opencode` |

---

## Regenerating These GIFs

These GIFs are generated using [VHS](https://github.com/charmbracelet/vhs). To regenerate:

```bash
# Generate all GIFs
just assets

# Generate a specific GIF
just asset help
just asset init
just asset inspect
just asset verify
just asset trust
```

See [README.md](README.md) for details on writing tape files.
