//! Catalog registry management
//!
//! Manages configured catalog registries and fetches indexes.
//! Includes caching with a 15-minute TTL to reduce network requests.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use super::CatalogIndex;

/// Default cache TTL (15 minutes)
pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(15 * 60);

/// Default official registry name
pub const DEFAULT_REGISTRY_NAME: &str = "official";

/// Default official registry URL
pub const DEFAULT_REGISTRY_URL: &str = "https://catalog.eqtylab.io/index.yaml";

/// A configured catalog registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    /// Registry name (e.g., "official", "mycompany")
    pub name: String,

    /// URL to the index.yaml file
    pub url: String,

    /// Whether this is the default registry
    #[serde(default)]
    pub is_default: bool,
}

/// Registry configuration file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// List of configured registries
    pub registries: Vec<Registry>,
}

/// Cached index metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedIndex {
    /// When the index was cached (Unix timestamp)
    pub cached_at: u64,

    /// The cached catalog index
    pub index: CatalogIndex,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            registries: vec![Registry {
                name: DEFAULT_REGISTRY_NAME.to_string(),
                url: DEFAULT_REGISTRY_URL.to_string(),
                is_default: true,
            }],
        }
    }
}

/// Manages catalog registries
pub struct RegistryManager {
    config: RegistryConfig,
    config_path: PathBuf,
}

impl RegistryManager {
    /// Load registry configuration from the default location
    pub fn load() -> Result<Self> {
        let config_path = Self::default_config_path()?;
        Self::load_from_path(config_path)
    }

    /// Load registry configuration from a specific path
    pub fn load_from_path(config_path: PathBuf) -> Result<Self> {
        let config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).with_context(|| {
                format!("Failed to read registry config: {}", config_path.display())
            })?;
            serde_yaml_ng::from_str(&content).with_context(|| {
                format!("Failed to parse registry config: {}", config_path.display())
            })?
        } else {
            RegistryConfig::default()
        };

        Ok(Self {
            config,
            config_path,
        })
    }

    /// Get the default config file path
    fn default_config_path() -> Result<PathBuf> {
        let config_dir = Self::config_dir()?;
        Ok(config_dir.join("registries.yaml"))
    }

    /// Get the config directory
    fn config_dir() -> Result<PathBuf> {
        let config_dir = directories::ProjectDirs::from("io", "eqtylab", "cupcake")
            .map(|dirs| dirs.config_dir().to_path_buf())
            .or_else(|| dirs::config_dir().map(|d| d.join("cupcake")))
            .context("Could not determine config directory")?;

        std::fs::create_dir_all(&config_dir).with_context(|| {
            format!(
                "Failed to create config directory: {}",
                config_dir.display()
            )
        })?;

        Ok(config_dir)
    }

    /// Get the cache directory
    fn cache_dir() -> Result<PathBuf> {
        let config_dir = Self::config_dir()?;
        let cache_dir = config_dir.join("cache");

        std::fs::create_dir_all(&cache_dir).with_context(|| {
            format!("Failed to create cache directory: {}", cache_dir.display())
        })?;

        Ok(cache_dir)
    }

    /// Get cache file path for a URL
    fn cache_path_for_url(url: &str) -> Result<PathBuf> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        let hash = hasher.finish();

        let cache_dir = Self::cache_dir()?;
        Ok(cache_dir.join(format!("index_{hash:016x}.yaml")))
    }

    /// Load cached index if valid
    fn load_cached_index(url: &str) -> Result<Option<CatalogIndex>> {
        let cache_path = Self::cache_path_for_url(url)?;

        if !cache_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&cache_path)
            .with_context(|| format!("Failed to read cache: {}", cache_path.display()))?;

        let cached: CachedIndex = serde_yaml_ng::from_str(&content)
            .with_context(|| format!("Failed to parse cache: {}", cache_path.display()))?;

        // Check TTL
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let age = Duration::from_secs(now.saturating_sub(cached.cached_at));

        if age > DEFAULT_CACHE_TTL {
            tracing::debug!("Cache expired for {} (age: {:?})", url, age);
            return Ok(None);
        }

        tracing::debug!("Using cached index for {} (age: {:?})", url, age);
        Ok(Some(cached.index))
    }

    /// Save index to cache
    fn save_to_cache(url: &str, index: &CatalogIndex) -> Result<()> {
        let cache_path = Self::cache_path_for_url(url)?;

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let cached = CachedIndex {
            cached_at: now,
            index: index.clone(),
        };

        let content = serde_yaml_ng::to_string(&cached).context("Failed to serialize cache")?;

        std::fs::write(&cache_path, content)
            .with_context(|| format!("Failed to write cache: {}", cache_path.display()))?;

        tracing::debug!("Saved index to cache: {}", cache_path.display());
        Ok(())
    }

    /// Clear all cached indexes
    pub fn clear_cache() -> Result<()> {
        let cache_dir = Self::cache_dir()?;

        for entry in std::fs::read_dir(&cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().map(|e| e == "yaml").unwrap_or(false) {
                std::fs::remove_file(&path)?;
            }
        }

        Ok(())
    }

    /// Save the current configuration
    pub fn save(&self) -> Result<()> {
        let content = serde_yaml_ng::to_string(&self.config)
            .context("Failed to serialize registry config")?;

        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&self.config_path, content).with_context(|| {
            format!(
                "Failed to write registry config: {}",
                self.config_path.display()
            )
        })?;

        Ok(())
    }

    /// Get all configured registries
    pub fn registries(&self) -> &[Registry] {
        &self.config.registries
    }

    /// Get a registry by name
    pub fn get_registry(&self, name: &str) -> Option<&Registry> {
        self.config.registries.iter().find(|r| r.name == name)
    }

    /// Get the default registry
    pub fn default_registry(&self) -> Option<&Registry> {
        self.config
            .registries
            .iter()
            .find(|r| r.is_default)
            .or_else(|| self.config.registries.first())
    }

    /// Add a new registry
    pub fn add_registry(&mut self, name: &str, url: &str) -> Result<()> {
        if self.config.registries.iter().any(|r| r.name == name) {
            anyhow::bail!("Registry '{}' already exists", name);
        }

        // Validate URL format
        if !url.starts_with("http://") && !url.starts_with("https://") {
            anyhow::bail!("Registry URL must start with http:// or https://");
        }

        self.config.registries.push(Registry {
            name: name.to_string(),
            url: url.to_string(),
            is_default: false,
        });

        Ok(())
    }

    /// Remove a registry by name
    pub fn remove_registry(&mut self, name: &str) -> Result<()> {
        if name == DEFAULT_REGISTRY_NAME {
            anyhow::bail!(
                "Cannot remove the default '{}' registry",
                DEFAULT_REGISTRY_NAME
            );
        }

        let initial_len = self.config.registries.len();
        self.config.registries.retain(|r| r.name != name);

        if self.config.registries.len() == initial_len {
            anyhow::bail!("Registry '{}' not found", name);
        }

        Ok(())
    }

    /// Fetch index from a single registry
    ///
    /// Uses cached index if available and valid (< 15 min old).
    /// Set `force_refresh` to bypass cache and fetch fresh data.
    #[cfg(feature = "catalog")]
    pub async fn fetch_index(
        &self,
        registry_name: &str,
        force_refresh: bool,
    ) -> Result<CatalogIndex> {
        let registry = self
            .get_registry(registry_name)
            .with_context(|| format!("Registry '{registry_name}' not found"))?;

        fetch_index_cached(&registry.url, force_refresh).await
    }

    /// Fetch and merge indexes from all configured registries
    ///
    /// Uses cached indexes if available and valid (< 15 min old).
    /// Set `force_refresh` to bypass cache and fetch fresh data.
    #[cfg(feature = "catalog")]
    pub async fn fetch_merged_index(&self, force_refresh: bool) -> Result<CatalogIndex> {
        let mut merged = CatalogIndex::new();

        for registry in &self.config.registries {
            match fetch_index_cached(&registry.url, force_refresh).await {
                Ok(index) => {
                    tracing::debug!(
                        "Fetched index from '{}': {} rulebooks",
                        registry.name,
                        index.rulebook_count()
                    );
                    merged.merge(index);
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch index from '{}': {}", registry.name, e);
                    // Continue with other registries
                }
            }
        }

        Ok(merged)
    }

    /// Stub for when catalog feature is disabled
    #[cfg(not(feature = "catalog"))]
    pub async fn fetch_index(
        &self,
        _registry_name: &str,
        _force_refresh: bool,
    ) -> Result<CatalogIndex> {
        anyhow::bail!("Catalog feature is not enabled. Rebuild with --features catalog")
    }

    /// Stub for when catalog feature is disabled
    #[cfg(not(feature = "catalog"))]
    pub async fn fetch_merged_index(&self, _force_refresh: bool) -> Result<CatalogIndex> {
        anyhow::bail!("Catalog feature is not enabled. Rebuild with --features catalog")
    }
}

/// Fetch a catalog index from a URL with caching
#[cfg(feature = "catalog")]
async fn fetch_index_cached(url: &str, force_refresh: bool) -> Result<CatalogIndex> {
    // Check cache first (unless force refresh)
    if !force_refresh {
        if let Ok(Some(cached)) = RegistryManager::load_cached_index(url) {
            return Ok(cached);
        }
    }

    // Fetch fresh index
    let index = fetch_index_from_url(url).await?;

    // Save to cache (ignore errors - caching is best effort)
    if let Err(e) = RegistryManager::save_to_cache(url, &index) {
        tracing::warn!("Failed to save index to cache: {}", e);
    }

    Ok(index)
}

/// Fetch a catalog index from a URL (no caching)
#[cfg(feature = "catalog")]
async fn fetch_index_from_url(url: &str) -> Result<CatalogIndex> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("cupcake/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch catalog index from {url}"))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch catalog index: HTTP {} from {}",
            response.status(),
            url
        );
    }

    let content = response
        .text()
        .await
        .context("Failed to read response body")?;

    CatalogIndex::from_yaml(&content)
        .with_context(|| format!("Failed to parse catalog index from {url}"))
}

#[cfg(test)]
mod registry_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = RegistryConfig::default();
        assert_eq!(config.registries.len(), 1);
        assert_eq!(config.registries[0].name, DEFAULT_REGISTRY_NAME);
        assert!(config.registries[0].is_default);
    }

    #[test]
    fn test_add_registry() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("registries.yaml");

        let mut manager = RegistryManager::load_from_path(config_path).unwrap();

        manager
            .add_registry("mycompany", "https://registry.mycompany.com/index.yaml")
            .unwrap();

        assert_eq!(manager.registries().len(), 2);
        assert!(manager.get_registry("mycompany").is_some());
    }

    #[test]
    fn test_add_duplicate_registry() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("registries.yaml");

        let mut manager = RegistryManager::load_from_path(config_path).unwrap();

        let result = manager.add_registry(DEFAULT_REGISTRY_NAME, "https://other.com/index.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_registry() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("registries.yaml");

        let mut manager = RegistryManager::load_from_path(config_path).unwrap();

        manager
            .add_registry("mycompany", "https://registry.mycompany.com/index.yaml")
            .unwrap();
        assert_eq!(manager.registries().len(), 2);

        manager.remove_registry("mycompany").unwrap();
        assert_eq!(manager.registries().len(), 1);
    }

    #[test]
    fn test_cannot_remove_default() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("registries.yaml");

        let mut manager = RegistryManager::load_from_path(config_path).unwrap();

        let result = manager.remove_registry(DEFAULT_REGISTRY_NAME);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot remove"));
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("registries.yaml");

        // Create and save
        {
            let mut manager = RegistryManager::load_from_path(config_path.clone()).unwrap();
            manager
                .add_registry("mycompany", "https://registry.mycompany.com/index.yaml")
                .unwrap();
            manager.save().unwrap();
        }

        // Load again
        {
            let manager = RegistryManager::load_from_path(config_path).unwrap();
            assert_eq!(manager.registries().len(), 2);
            assert!(manager.get_registry("mycompany").is_some());
        }
    }

    #[test]
    fn test_cache_path_for_url() {
        // Test that different URLs get different cache paths
        let path1 = RegistryManager::cache_path_for_url("https://example.com/index.yaml").unwrap();
        let path2 = RegistryManager::cache_path_for_url("https://other.com/index.yaml").unwrap();

        assert_ne!(path1, path2);
        assert!(path1.to_string_lossy().contains("index_"));
        assert!(path1.to_string_lossy().ends_with(".yaml"));
    }

    #[test]
    fn test_cache_save_and_load() {
        let url = "https://test-cache.example.com/index.yaml";

        // Create a test index
        let mut index = CatalogIndex::new();
        index.entries.insert(
            "test-rulebook".to_string(),
            vec![super::super::IndexEntry {
                name: "test-rulebook".to_string(),
                version: "1.0.0".to_string(),
                description: "Test rulebook".to_string(),
                harnesses: vec!["claude".to_string()],
                keywords: vec![],
                digest: None,
                created: None,
                urls: vec![],
                deprecated: false,
            }],
        );

        // Save to cache
        RegistryManager::save_to_cache(url, &index).unwrap();

        // Load from cache
        let loaded = RegistryManager::load_cached_index(url).unwrap();
        assert!(loaded.is_some());

        let loaded_index = loaded.unwrap();
        assert_eq!(loaded_index.rulebook_count(), 1);
        assert!(loaded_index.get_latest("test-rulebook").is_some());

        // Clean up
        let cache_path = RegistryManager::cache_path_for_url(url).unwrap();
        let _ = std::fs::remove_file(cache_path);
    }

    #[test]
    fn test_cache_miss_on_nonexistent() {
        let url = "https://nonexistent-cache-test.example.com/index.yaml";
        let loaded = RegistryManager::load_cached_index(url).unwrap();
        assert!(loaded.is_none());
    }
}
