//! Cupcake Catalog CLI commands
//!
//! Provides commands for discovering, installing, and managing
//! rulebooks from the Cupcake Catalog.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tabled::{
    settings::{object::Rows, Alignment, Modify, Style},
    Table, Tabled,
};

use cupcake_core::catalog::{CatalogLock, IndexEntry, Installer, RegistryManager};

/// Catalog subcommand for browsing and managing rulebooks
#[derive(Parser, Debug)]
pub struct CatalogCommand {
    #[clap(subcommand)]
    pub command: CatalogSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum CatalogSubcommand {
    /// Manage catalog repositories
    Repo {
        #[clap(subcommand)]
        command: RepoCommand,
    },

    /// Search for rulebooks in the catalog
    Search {
        /// Search query (searches name, description, keywords)
        query: Option<String>,

        /// Filter by harness type
        #[clap(long)]
        harness: Option<String>,

        /// Output results as JSON
        #[clap(long)]
        json: bool,

        /// Force refresh of catalog index (bypass cache)
        #[clap(long)]
        refresh: bool,
    },

    /// Show detailed information about a rulebook
    Show {
        /// Rulebook name
        name: String,

        /// Output as JSON
        #[clap(long)]
        json: bool,

        /// Force refresh of catalog index (bypass cache)
        #[clap(long)]
        refresh: bool,
    },

    /// Install a rulebook from the catalog
    Install {
        /// Rulebook name (optionally with @version, e.g., security-hardened@1.2.0)
        name: String,

        /// Specific version to install
        #[clap(long)]
        version: Option<String>,

        /// Force refresh of catalog index (bypass cache)
        #[clap(long)]
        refresh: bool,
    },

    /// List installed catalog rulebooks
    List,

    /// Upgrade installed rulebooks to newer versions
    Upgrade {
        /// Rulebook name (upgrades all if omitted)
        name: Option<String>,

        /// Show what would be upgraded without making changes
        #[clap(long)]
        dry_run: bool,

        /// Force refresh of catalog index (bypass cache)
        #[clap(long)]
        refresh: bool,
    },

    /// Uninstall a catalog rulebook
    Uninstall {
        /// Rulebook name
        name: String,
    },

    /// Validate a local rulebook before publishing
    Lint {
        /// Path to rulebook directory
        path: std::path::PathBuf,
    },

    /// Package a rulebook for distribution
    Package {
        /// Path to rulebook directory
        path: std::path::PathBuf,

        /// Output directory for tarball (defaults to current directory)
        #[clap(long, short, default_value = ".")]
        output: std::path::PathBuf,
    },
}

#[derive(Subcommand, Debug)]
pub enum RepoCommand {
    /// Add a catalog repository
    Add {
        /// Repository name (e.g., "mycompany")
        name: String,
        /// Repository URL (must serve index.yaml)
        url: String,
    },

    /// List configured repositories
    List,

    /// Remove a repository
    Remove {
        /// Repository name
        name: String,
    },
}

impl CatalogCommand {
    pub async fn execute(self) -> Result<()> {
        match self.command {
            CatalogSubcommand::Repo { command } => execute_repo_command(command).await,
            CatalogSubcommand::Search {
                query,
                harness,
                json,
                refresh,
            } => execute_search(query, harness, json, refresh).await,
            CatalogSubcommand::Show {
                name,
                json,
                refresh,
            } => execute_show(&name, json, refresh).await,
            CatalogSubcommand::Install {
                name,
                version,
                refresh,
            } => execute_install(&name, version.as_deref(), refresh).await,
            CatalogSubcommand::List => execute_list().await,
            CatalogSubcommand::Upgrade {
                name,
                dry_run,
                refresh,
            } => execute_upgrade(name.as_deref(), dry_run, refresh).await,
            CatalogSubcommand::Uninstall { name } => execute_uninstall(&name).await,
            CatalogSubcommand::Lint { path } => execute_lint(&path).await,
            CatalogSubcommand::Package { path, output } => execute_package(&path, &output).await,
        }
    }
}

async fn execute_repo_command(command: RepoCommand) -> Result<()> {
    let mut manager = RegistryManager::load()?;

    match command {
        RepoCommand::Add { name, url } => {
            manager.add_registry(&name, &url)?;
            manager.save()?;
            println!("Added repository '{name}' -> {url}");
        }
        RepoCommand::List => {
            println!("Configured repositories:\n");
            for registry in manager.registries() {
                let default_marker = if registry.is_default {
                    " (default)"
                } else {
                    ""
                };
                println!("  {} -> {}{}", registry.name, registry.url, default_marker);
            }
        }
        RepoCommand::Remove { name } => {
            manager.remove_registry(&name)?;
            manager.save()?;
            println!("Removed repository '{name}'");
        }
    }

    Ok(())
}

/// Table row for search results
#[derive(Tabled)]
struct SearchResultRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "Harnesses")]
    harnesses: String,
    #[tabled(rename = "Description")]
    description: String,
}

async fn execute_search(
    query: Option<String>,
    harness: Option<String>,
    json_output: bool,
    force_refresh: bool,
) -> Result<()> {
    let manager = RegistryManager::load()?;

    if force_refresh {
        println!("Fetching catalog index (refreshing cache)...");
    } else {
        println!("Fetching catalog index...");
    }
    let index = manager.fetch_merged_index(force_refresh).await?;

    // Get all entries, then filter
    let mut results: Vec<&IndexEntry> = if let Some(ref q) = query {
        index.search(q)
    } else {
        index.list_all()
    };

    // Filter by harness if specified
    if let Some(ref h) = harness {
        results.retain(|entry| entry.harnesses.contains(&h.to_string()));
    }

    // Sort by name
    results.sort_by(|a, b| a.name.cmp(&b.name));

    if results.is_empty() {
        println!("\nNo rulebooks found.");
        return Ok(());
    }

    if json_output {
        let json_results: Vec<serde_json::Value> = results
            .iter()
            .map(|entry| {
                serde_json::json!({
                    "name": entry.name,
                    "version": entry.version,
                    "description": entry.description,
                    "harnesses": entry.harnesses,
                    "keywords": entry.keywords,
                    "deprecated": entry.deprecated,
                })
            })
            .collect();

        println!("{}", serde_json::to_string_pretty(&json_results)?);
    } else {
        println!("\nFound {} rulebook(s):\n", results.len());

        let table_rows: Vec<SearchResultRow> = results
            .iter()
            .map(|entry| {
                let desc = entry.short_description();
                let truncated_desc = if desc.len() > 50 {
                    format!("{}...", &desc[..47])
                } else {
                    desc.to_string()
                };

                SearchResultRow {
                    name: entry.name.clone(),
                    version: entry.version.clone(),
                    harnesses: entry.harnesses_display(),
                    description: truncated_desc,
                }
            })
            .collect();

        let table = Table::new(&table_rows)
            .with(Style::rounded())
            .with(Modify::new(Rows::first()).with(Alignment::center()))
            .to_string();

        println!("{table}");
    }

    Ok(())
}

async fn execute_show(name: &str, json_output: bool, force_refresh: bool) -> Result<()> {
    let manager = RegistryManager::load()?;

    if force_refresh {
        println!("Fetching catalog index (refreshing cache)...");
    } else {
        println!("Fetching catalog index...");
    }
    let index = manager.fetch_merged_index(force_refresh).await?;

    let versions = index
        .get_versions(name)
        .context(format!("Rulebook '{name}' not found in catalog"))?;

    let latest = versions.first().context("No versions available")?;

    if json_output {
        let output = serde_json::json!({
            "name": latest.name,
            "latest_version": latest.version,
            "description": latest.description,
            "harnesses": latest.harnesses,
            "keywords": latest.keywords,
            "deprecated": latest.deprecated,
            "available_versions": versions.iter().map(|v| &v.version).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("Rulebook: {}", latest.name);
        println!("Latest:   v{}", latest.version);
        println!("Harnesses: {}", latest.harnesses.join(", "));

        if !latest.keywords.is_empty() {
            println!("Keywords: {}", latest.keywords.join(", "));
        }

        if latest.deprecated {
            println!();
            println!("WARNING: This rulebook is deprecated!");
        }

        println!();
        println!("Description:");
        for line in latest.description.lines() {
            println!("  {line}");
        }

        println!();
        println!("Installation:");
        println!("  cupcake catalog install {}", latest.name);
        println!(
            "  cupcake catalog install {}@{}",
            latest.name, latest.version
        );

        println!();
        println!("Available versions:");
        for (i, v) in versions.iter().take(10).enumerate() {
            let marker = if i == 0 { " (latest)" } else { "" };
            let deprecated = if v.deprecated { " [DEPRECATED]" } else { "" };
            println!("  v{}{}{}", v.version, marker, deprecated);
        }

        if versions.len() > 10 {
            println!("  ... and {} more", versions.len() - 10);
        }
    }

    Ok(())
}

async fn execute_install(name: &str, version: Option<&str>, force_refresh: bool) -> Result<()> {
    // Parse name@version syntax
    let (rulebook_name, requested_version) = if name.contains('@') {
        let parts: Vec<&str> = name.splitn(2, '@').collect();
        (parts[0], Some(parts[1]))
    } else {
        (name, version)
    };

    let manager = RegistryManager::load()?;

    if force_refresh {
        println!("Fetching catalog index (refreshing cache)...");
    } else {
        println!("Fetching catalog index...");
    }
    let index = manager.fetch_merged_index(force_refresh).await?;

    // Ensure the rulebook exists
    if !index.entries.contains_key(rulebook_name) {
        anyhow::bail!("Rulebook '{}' not found in catalog", rulebook_name);
    }

    // Resolve version (supports exact, ^caret, ~tilde, or latest)
    let version_spec = requested_version.unwrap_or("");
    let entry = index
        .resolve_version(rulebook_name, version_spec)
        .context(format!(
            "No version matching '{version_spec}' found for '{rulebook_name}'.\n\
             Supported formats: 1.2.0 (exact), ^1.2 (compatible), ~1.2 (patch-level), latest"
        ))?;

    // Show what version was resolved for non-exact specifiers
    if let Some(spec) = requested_version {
        if spec.starts_with('^') || spec.starts_with('~') {
            println!("\nResolved {} -> v{}", spec, entry.version);
        }
    }

    println!(
        "\nInstalling {} v{} (harnesses: {})...\n",
        entry.name,
        entry.version,
        entry.harnesses.join(", ")
    );

    // Download and install
    let installer = Installer::new()?;
    installer.install(entry).await?;

    // Update lock file
    let mut lock = CatalogLock::load_or_default()?;
    lock.add_installed(entry, "official"); // TODO: track actual repo
    lock.save()?;

    println!("\nInstalled {} v{}", entry.name, entry.version);
    println!("Location: .cupcake/catalog/{}/", entry.name);

    Ok(())
}

/// Table row for installed rulebooks
#[derive(Tabled)]
struct InstalledRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "Harnesses")]
    harnesses: String,
    #[tabled(rename = "Installed")]
    installed_at: String,
}

async fn execute_list() -> Result<()> {
    let lock = CatalogLock::load_or_default()?;

    if lock.installed.is_empty() {
        println!("No catalog rulebooks installed.");
        println!("\nRun 'cupcake catalog search' to find available rulebooks.");
        return Ok(());
    }

    println!("Installed catalog rulebooks:\n");

    let table_rows: Vec<InstalledRow> = lock
        .installed
        .iter()
        .map(|entry| {
            // Parse and format the installed_at timestamp
            let installed_display = entry
                .installed_at
                .split('T')
                .next()
                .unwrap_or(&entry.installed_at)
                .to_string();

            InstalledRow {
                name: entry.name.clone(),
                version: format!("v{}", entry.version),
                harnesses: entry.harnesses.join(", "),
                installed_at: installed_display,
            }
        })
        .collect();

    let table = Table::new(&table_rows)
        .with(Style::rounded())
        .with(Modify::new(Rows::first()).with(Alignment::center()))
        .to_string();

    println!("{table}");

    Ok(())
}

async fn execute_uninstall(name: &str) -> Result<()> {
    let mut lock = CatalogLock::load_or_default()?;

    if !lock.is_installed(name) {
        println!("Rulebook '{name}' is not installed.");
        return Ok(());
    }

    // Get version info before uninstalling
    let version = lock
        .get_installed(name)
        .map(|e| e.version.clone())
        .unwrap_or_default();

    // Remove directory
    let installer = Installer::new()?;
    installer.uninstall(name)?;

    // Update lock
    lock.remove_installed(name);
    lock.save()?;

    println!("Uninstalled '{name}' v{version}");

    Ok(())
}

/// Table row for upgrade preview
#[derive(Tabled)]
struct UpgradeRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Current")]
    current: String,
    #[tabled(rename = "Latest")]
    latest: String,
    #[tabled(rename = "Status")]
    status: String,
}

/// Represents an upgrade to be performed
struct UpgradePlan {
    name: String,
    current_version: String,
    repository: String,
    latest: IndexEntry,
}

async fn execute_upgrade(name: Option<&str>, dry_run: bool, force_refresh: bool) -> Result<()> {
    let manager = RegistryManager::load()?;
    let mut lock = CatalogLock::load_or_default()?;

    if lock.installed.is_empty() {
        println!("No catalog rulebooks installed.");
        println!("\nRun 'cupcake catalog search' to find available rulebooks.");
        return Ok(());
    }

    if force_refresh {
        println!("Fetching catalog index (refreshing cache)...");
    } else {
        println!("Fetching catalog index...");
    }
    let index = manager.fetch_merged_index(force_refresh).await?;

    // Build list of installed to check
    let to_check: Vec<_> = if let Some(n) = name {
        lock.installed
            .iter()
            .filter(|e| e.name == n)
            .cloned()
            .collect()
    } else {
        lock.installed.clone()
    };

    if to_check.is_empty() {
        if let Some(n) = name {
            println!("Rulebook '{n}' is not installed.");
        }
        return Ok(());
    }

    // Build upgrade plan (owned data to avoid borrow issues)
    let mut upgrade_plan: Vec<UpgradePlan> = Vec::new();
    let mut not_in_catalog: Vec<String> = Vec::new();

    for installed in &to_check {
        match index.get_latest(&installed.name) {
            Some(latest) => {
                let current_ver = semver::Version::parse(&installed.version).ok();
                let latest_ver = semver::Version::parse(&latest.version).ok();

                let needs_upgrade = match (current_ver, latest_ver) {
                    (Some(c), Some(l)) => l > c,
                    _ => latest.version != installed.version,
                };

                if needs_upgrade {
                    upgrade_plan.push(UpgradePlan {
                        name: installed.name.clone(),
                        current_version: installed.version.clone(),
                        repository: installed.repository.clone(),
                        latest: latest.clone(),
                    });
                }
            }
            None => {
                not_in_catalog.push(installed.name.clone());
            }
        }
    }

    // Display results
    if upgrade_plan.is_empty() && not_in_catalog.is_empty() {
        println!("\nAll installed rulebooks are up to date.");
        return Ok(());
    }

    if !upgrade_plan.is_empty() {
        let table_rows: Vec<UpgradeRow> = upgrade_plan
            .iter()
            .map(|plan| UpgradeRow {
                name: plan.name.clone(),
                current: format!("v{}", plan.current_version),
                latest: format!("v{}", plan.latest.version),
                status: "upgrade available".to_string(),
            })
            .collect();

        println!("\nUpgrades available:\n");
        let table = Table::new(&table_rows)
            .with(Style::rounded())
            .with(Modify::new(Rows::first()).with(Alignment::center()))
            .to_string();
        println!("{table}");
    }

    for name in &not_in_catalog {
        println!("\nWarning: '{name}' not found in catalog (may have been removed)");
    }

    if dry_run {
        println!("\n--dry-run: No changes made.");
        return Ok(());
    }

    if upgrade_plan.is_empty() {
        return Ok(());
    }

    // Perform upgrades
    println!("\nUpgrading...\n");
    let installer = Installer::new()?;

    for plan in &upgrade_plan {
        println!(
            "Upgrading {} from v{} to v{}...",
            plan.name, plan.current_version, plan.latest.version
        );

        installer.install(&plan.latest).await?;
        lock.add_installed(&plan.latest, &plan.repository);
    }

    lock.save()?;

    println!(
        "\nUpgrade complete. {} rulebook(s) upgraded.",
        upgrade_plan.len()
    );

    Ok(())
}

async fn execute_lint(path: &std::path::Path) -> Result<()> {
    use cupcake_core::catalog::RulebookManifest;

    println!("Validating rulebook at {path:?}...\n");

    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Check manifest exists
    let manifest_path = path.join("manifest.yaml");
    if !manifest_path.exists() {
        errors.push("manifest.yaml not found".to_string());
        print_validation_results(&errors, &warnings);
        return Ok(());
    }

    // Parse and validate manifest
    match RulebookManifest::from_file(&manifest_path) {
        Ok(manifest) => {
            // Validate manifest fields
            if let Err(e) = manifest.validate() {
                errors.push(format!("Manifest validation failed: {e}"));
            }

            // Check policies exist for declared harnesses
            let policies_dir = path.join("policies");
            if !policies_dir.exists() {
                errors.push("policies/ directory not found".to_string());
            } else {
                for harness in &manifest.metadata.harnesses {
                    let harness_dir = policies_dir.join(harness);
                    if !harness_dir.exists() {
                        errors.push(format!("Missing policies directory for harness: {harness}"));
                        continue;
                    }

                    // Check for system/evaluate.rego
                    let system_eval = harness_dir.join("system").join("evaluate.rego");
                    if !system_eval.exists() {
                        errors.push(format!(
                            "Missing system/evaluate.rego for harness: {harness}"
                        ));
                    }

                    // Check that at least some .rego files exist
                    let rego_files = count_rego_files(&harness_dir);
                    if rego_files == 0 {
                        errors.push(format!("No .rego files found for harness: {harness}"));
                    }
                }
            }

            // Validate Rego namespaces
            if policies_dir.exists() {
                if let Err(e) = validate_rego_namespaces(path, &manifest.metadata.name) {
                    errors.push(format!("Namespace validation failed: {e}"));
                }
            }

            // Check for README
            if !path.join("README.md").exists() {
                warnings.push("README.md not found (recommended)".to_string());
            }

            // Check for CHANGELOG
            if !path.join("CHANGELOG.md").exists() {
                warnings.push("CHANGELOG.md not found (recommended)".to_string());
            }

            // Print rulebook info
            println!("Rulebook: {}", manifest.metadata.name);
            println!("Version:  {}", manifest.metadata.version);
            println!("Harnesses: {}", manifest.metadata.harnesses.join(", "));
            println!();
        }
        Err(e) => {
            errors.push(format!("Failed to parse manifest.yaml: {e}"));
        }
    }

    print_validation_results(&errors, &warnings);

    if !errors.is_empty() {
        std::process::exit(1);
    }

    Ok(())
}

fn count_rego_files(dir: &std::path::Path) -> usize {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file()
                && e.path()
                    .extension()
                    .map(|ext| ext == "rego")
                    .unwrap_or(false)
        })
        .count()
}

fn validate_rego_namespaces(rulebook_path: &std::path::Path, rulebook_name: &str) -> Result<()> {
    let policies_dir = rulebook_path.join("policies");
    let expected_prefix = format!("cupcake.catalog.{}", rulebook_name.replace('-', "_"));

    for entry in walkdir::WalkDir::new(&policies_dir) {
        let entry = entry?;
        if !entry.path().is_file() {
            continue;
        }

        if entry
            .path()
            .extension()
            .map(|ext| ext != "rego")
            .unwrap_or(true)
        {
            continue;
        }

        let content = std::fs::read_to_string(entry.path())?;

        // Find package declaration
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("package ") {
                let package_name = trimmed.strip_prefix("package ").unwrap_or("").trim();

                // Check namespace prefix
                if !package_name.starts_with(&expected_prefix) {
                    anyhow::bail!(
                        "Policy at {:?} has invalid namespace '{}'. Expected prefix '{}'",
                        entry.path(),
                        package_name,
                        expected_prefix
                    );
                }
                break;
            }
        }
    }

    Ok(())
}

fn print_validation_results(errors: &[String], warnings: &[String]) {
    if errors.is_empty() && warnings.is_empty() {
        println!("Rulebook is valid.");
        return;
    }

    for error in errors {
        println!("ERROR: {error}");
    }

    for warning in warnings {
        println!("WARNING: {warning}");
    }

    println!();
    if errors.is_empty() {
        println!("Validation passed with {} warning(s).", warnings.len());
    } else {
        println!(
            "Validation failed with {} error(s) and {} warning(s).",
            errors.len(),
            warnings.len()
        );
    }
}

async fn execute_package(path: &std::path::Path, output: &std::path::Path) -> Result<()> {
    use cupcake_core::catalog::RulebookManifest;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use sha2::{Digest, Sha256};
    use tar::Builder;

    println!("Packaging rulebook at {path:?}...\n");

    // Validate first
    let manifest_path = path.join("manifest.yaml");
    if !manifest_path.exists() {
        anyhow::bail!("manifest.yaml not found. Run 'cupcake catalog lint' to validate.");
    }

    let manifest = RulebookManifest::from_file(&manifest_path)?;
    manifest.validate()?;

    let name = &manifest.metadata.name;
    let version = &manifest.metadata.version;

    // Create tarball filename
    let tarball_name = format!("{name}-{version}.tar.gz");
    let tarball_path = output.join(&tarball_name);

    // Ensure output directory exists
    std::fs::create_dir_all(output)?;

    // Create tarball
    let tarball_file = std::fs::File::create(&tarball_path)
        .with_context(|| format!("Failed to create {tarball_path:?}"))?;

    let encoder = GzEncoder::new(tarball_file, Compression::default());
    let mut builder = Builder::new(encoder);

    // Add rulebook directory to tarball with the rulebook name as the root
    builder
        .append_dir_all(name, path)
        .context("Failed to add files to tarball")?;

    let encoder = builder.into_inner().context("Failed to finalize tarball")?;
    encoder.finish().context("Failed to compress tarball")?;

    // Calculate digest
    let tarball_bytes = std::fs::read(&tarball_path)?;
    let digest = format!("sha256:{:x}", Sha256::digest(&tarball_bytes));

    println!("Created: {tarball_path:?}");
    println!("Size:    {} bytes", tarball_bytes.len());
    println!("Digest:  {digest}");
    println!();
    println!("Rulebook: {name} v{version}");
    println!("Harnesses: {}", manifest.metadata.harnesses.join(", "));

    Ok(())
}
