//! Integration tests for the catalog module

#[cfg(test)]
mod integration_tests {
    use crate::catalog::{CatalogIndex, IndexEntry, RulebookManifest};

    /// Test that we can round-trip a manifest through YAML
    #[test]
    fn test_manifest_yaml_roundtrip() {
        let yaml = r#"
apiVersion: cupcake.dev/v1
kind: Rulebook
metadata:
  name: security-hardened
  version: 1.0.0
  description: Production-ready security policies
  harnesses:
    - claude
    - cursor
    - opencode
    - factory
  keywords:
    - security
    - production
  license: Apache-2.0
  maintainers:
    - name: EQTY Lab
      email: support@eqtylab.io
spec:
  cupcakeVersion: ">=0.5.0"
"#;

        let manifest = RulebookManifest::from_yaml(yaml).unwrap();
        manifest.validate().unwrap();

        assert_eq!(manifest.metadata.name, "security-hardened");
        assert_eq!(manifest.metadata.harnesses.len(), 4);
        assert_eq!(manifest.rego_name(), "security_hardened");
        assert_eq!(
            manifest.namespace_prefix(),
            "cupcake.catalog.security_hardened"
        );
    }

    /// Test index search and filtering
    #[test]
    fn test_index_search_and_filter() {
        let mut index = CatalogIndex::new();

        // Add some entries
        index.entries.insert(
            "security-hardened".to_string(),
            vec![IndexEntry {
                name: "security-hardened".to_string(),
                version: "1.0.0".to_string(),
                description: "Production security policies".to_string(),
                harnesses: vec!["claude".to_string(), "cursor".to_string()],
                keywords: vec!["security".to_string(), "enterprise".to_string()],
                digest: None,
                created: None,
                urls: vec![],
                deprecated: false,
            }],
        );

        index.entries.insert(
            "git-workflow".to_string(),
            vec![IndexEntry {
                name: "git-workflow".to_string(),
                version: "0.5.0".to_string(),
                description: "Git best practices".to_string(),
                harnesses: vec!["claude".to_string(), "opencode".to_string()],
                keywords: vec!["git".to_string(), "workflow".to_string()],
                digest: None,
                created: None,
                urls: vec![],
                deprecated: false,
            }],
        );

        // Test search
        let results = index.search("security");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "security-hardened");

        let results = index.search("enterprise");
        assert_eq!(results.len(), 1);

        let results = index.search("workflow");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "git-workflow");

        // Test harness filter
        let results = index.filter_by_harness("claude");
        assert_eq!(results.len(), 2);

        let results = index.filter_by_harness("cursor");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "security-hardened");

        let results = index.filter_by_harness("opencode");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "git-workflow");

        let results = index.filter_by_harness("factory");
        assert_eq!(results.len(), 0);
    }

    /// Test that version sorting works correctly
    #[test]
    fn test_version_sorting() {
        let mut index = CatalogIndex::new();

        // Add versions in non-sorted order
        index.entries.insert(
            "test".to_string(),
            vec![
                IndexEntry {
                    name: "test".to_string(),
                    version: "1.0.0".to_string(),
                    description: "Test".to_string(),
                    harnesses: vec!["claude".to_string()],
                    keywords: vec![],
                    digest: None,
                    created: None,
                    urls: vec![],
                    deprecated: false,
                },
                IndexEntry {
                    name: "test".to_string(),
                    version: "2.0.0".to_string(),
                    description: "Test".to_string(),
                    harnesses: vec!["claude".to_string()],
                    keywords: vec![],
                    digest: None,
                    created: None,
                    urls: vec![],
                    deprecated: false,
                },
                IndexEntry {
                    name: "test".to_string(),
                    version: "1.5.0".to_string(),
                    description: "Test".to_string(),
                    harnesses: vec!["claude".to_string()],
                    keywords: vec![],
                    digest: None,
                    created: None,
                    urls: vec![],
                    deprecated: false,
                },
            ],
        );

        // Merge with empty to trigger sorting
        index.merge(CatalogIndex::new());

        let versions = index.get_versions("test").unwrap();
        assert_eq!(versions[0].version, "2.0.0"); // Newest first
        assert_eq!(versions[1].version, "1.5.0");
        assert_eq!(versions[2].version, "1.0.0");

        // Latest should be newest
        assert_eq!(index.get_latest("test").unwrap().version, "2.0.0");
    }
}
