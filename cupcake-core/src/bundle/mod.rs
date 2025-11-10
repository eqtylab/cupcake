//! Governance bundle support for Cupcake
//!
//! This module provides functionality to load and integrate governance bundles
//! from the governance-service, enabling centralized policy distribution while
//! preserving local overrides and builtin abstractions.

pub mod governance;

pub use governance::{
    Annotation, Author, BundleManifest, GovernanceBundle, GovernanceBundleLoader, WasmModule,
};
