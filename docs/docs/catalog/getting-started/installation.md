---
title: "Installation"
description: "Install and configure Cupcake Catalog"
---

# Installation

Cupcake Catalog is built into Cupcake. If you have Cupcake installed, you already have access to the catalog commands.

## Requirements

- Cupcake v0.3.0 or later
- Network access to fetch catalog indexes and download rulebooks

## Verify Installation

Check that catalog commands are available:

```bash
cupcake catalog --help
```

You should see output showing all available subcommands:

```
Browse and manage rulebooks from the Cupcake Catalog

Usage: cupcake catalog <COMMAND>

Commands:
  repo       Manage catalog repositories
  search     Search for rulebooks in the catalog
  show       Show detailed information about a rulebook
  install    Install a rulebook from the catalog
  list       List installed catalog rulebooks
  upgrade    Upgrade installed rulebooks to newer versions
  uninstall  Uninstall a catalog rulebook
  lint       Validate a local rulebook before publishing
  package    Package a rulebook for distribution
```

## Default Registry

By default, Cupcake Catalog uses the official EQTY Lab registry:

```
https://catalog.eqtylab.io/index.yaml
```

This is configured automatically and requires no setup.

## Adding Custom Registries

You can add additional registries to access private or organization-specific rulebooks:

```bash
# Add a custom registry
cupcake catalog repo add mycompany https://registry.mycompany.com/index.yaml

# List configured registries
cupcake catalog repo list

# Remove a registry
cupcake catalog repo remove mycompany
```

!!! note "Registry Merging"
When multiple registries are configured, their indexes are merged. If the same rulebook exists in multiple registries, versions are combined and sorted by version number.

## Index Caching

Catalog indexes are cached locally for 15 minutes to reduce network requests. To bypass the cache and fetch fresh data:

```bash
cupcake catalog search --refresh
cupcake catalog show security-hardened --refresh
cupcake catalog install security-hardened --refresh
```

## Offline Mode

If you're offline, Cupcake will use cached index data if available. Installed rulebooks work entirely offline once downloaded.
