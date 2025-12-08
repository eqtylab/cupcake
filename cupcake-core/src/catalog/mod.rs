//! Cupcake Catalog - Rulebook discovery and management
//!
//! This module provides functionality for discovering, installing,
//! and managing rulebooks from the Cupcake Catalog.
//!
//! # Overview
//!
//! The catalog system allows users to:
//! - Search and browse available rulebooks
//! - Install rulebooks as overlays in `.cupcake/catalog/`
//! - Manage multiple catalog registries
//! - Track installed versions via `catalog.lock`
//!
//! # Architecture
//!
//! ```text
//! Registry (GitHub Pages)
//!     │
//!     ├── index.yaml        ← Lists all rulebooks and versions
//!     └── releases/*.tar.gz ← Packaged rulebooks
//!            │
//!            ▼
//!     Cupcake CLI
//!            │
//!            ▼
//!     .cupcake/catalog/     ← Installed rulebooks
//!     .cupcake/catalog.lock ← Version tracking
//! ```

mod index;
mod installer;
mod lock;
mod manifest;
mod registry;

pub use index::{CatalogIndex, IndexEntry};
pub use installer::Installer;
pub use lock::{CatalogLock, InstalledRulebook};
pub use manifest::{Maintainer, ManifestMetadata, ManifestSpec, RulebookManifest};
pub use registry::{
    Registry, RegistryConfig, RegistryManager, DEFAULT_REGISTRY_NAME, DEFAULT_REGISTRY_URL,
};

#[cfg(test)]
mod tests;
