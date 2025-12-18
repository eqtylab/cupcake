#[cfg(test)]
mod tests {
    use cupcake_core::engine::rulebook::Rulebook;

    #[test]
    fn test_global_builtins_configuration_parsing() {
        let yaml_content = r#"
signals: {}
actions: {}

builtins:
  # Test system protection configuration
  system_protection:
    enabled: true
    additional_paths: 
      - "/custom/protected/path"
    message: "Custom system protection message"
  
  # Test sensitive data protection configuration
  sensitive_data_protection:
    enabled: true
    additional_patterns:
      - "*.secret"
      - "*.apikey"
  
  # Test cupcake exec protection configuration
  cupcake_exec_protection:
    enabled: true
    message: "No direct cupcake execution allowed"
"#;

        // Parse the Rulebook
        let rulebook: Rulebook =
            serde_yaml_ng::from_str(yaml_content).expect("Failed to parse YAML");

        let config = rulebook.builtins;

        // Verify system_protection
        assert!(config.system_protection.is_some());
        let sys_prot = config.system_protection.as_ref().unwrap();
        assert!(sys_prot.enabled);
        assert_eq!(sys_prot.additional_paths.len(), 1);
        assert_eq!(sys_prot.additional_paths[0], "/custom/protected/path");
        assert_eq!(sys_prot.message, "Custom system protection message");

        // Verify sensitive_data_protection
        assert!(config.sensitive_data_protection.is_some());
        let sens_data = config.sensitive_data_protection.as_ref().unwrap();
        assert!(sens_data.enabled);
        assert_eq!(sens_data.additional_patterns.len(), 2);
        assert_eq!(sens_data.additional_patterns[0], "*.secret");
        assert_eq!(sens_data.additional_patterns[1], "*.apikey");

        // Verify cupcake_exec_protection
        assert!(config.cupcake_exec_protection.is_some());
        let exec_prot = config.cupcake_exec_protection.as_ref().unwrap();
        assert!(exec_prot.enabled);
        assert_eq!(exec_prot.message, "No direct cupcake execution allowed");

        // Test enabled_builtins method
        let enabled = config.enabled_builtins();
        assert!(enabled.contains(&"system_protection".to_string()));
        assert!(enabled.contains(&"sensitive_data_protection".to_string()));
        assert!(enabled.contains(&"cupcake_exec_protection".to_string()));

        // Test any_enabled
        assert!(config.any_enabled());
    }

    #[test]
    fn test_global_builtins_default_values() {
        let yaml_content = r#"
signals: {}
actions: {}

builtins:
  system_protection:
    enabled: true
  sensitive_data_protection:
    enabled: false
  cupcake_exec_protection: {}
"#;

        let rulebook: Rulebook =
            serde_yaml_ng::from_str(yaml_content).expect("Failed to parse YAML");

        let config = rulebook.builtins;

        // System protection with defaults
        let sys_prot = config.system_protection.as_ref().unwrap();
        assert!(sys_prot.enabled);
        assert!(sys_prot.additional_paths.is_empty());
        assert_eq!(sys_prot.message, "Access to critical system path blocked");

        // Sensitive data protection disabled
        let sens_data = config.sensitive_data_protection.as_ref().unwrap();
        assert!(!sens_data.enabled);
        assert!(sens_data.additional_patterns.is_empty());

        // Cupcake exec protection with all defaults
        let exec_prot = config.cupcake_exec_protection.as_ref().unwrap();
        assert!(!exec_prot.enabled); // Should default to false (must be explicitly enabled)
        assert_eq!(
            exec_prot.message,
            "Direct execution of Cupcake binary is not permitted"
        );

        // Test enabled_builtins - only system_protection is enabled (explicit in YAML)
        let enabled = config.enabled_builtins();
        assert!(enabled.contains(&"system_protection".to_string()));
        assert!(!enabled.contains(&"sensitive_data_protection".to_string()));
        assert!(!enabled.contains(&"cupcake_exec_protection".to_string())); // Not enabled by default
    }

    #[test]
    fn test_global_builtins_signal_generation() {
        // After refactoring, global builtins (system_protection, sensitive_data_protection,
        // cupcake_exec_protection) no longer generate signals. They use builtin_config instead
        // to avoid unnecessary shell process spawning for static configuration values.
        let yaml_content = r#"
signals: {}
actions: {}

builtins:
  system_protection:
    additional_paths: 
      - "/opt/custom"
      - "/usr/local/special"
  sensitive_data_protection:
    additional_patterns:
      - "*.privatekey"
      - "vault-*"
  cupcake_exec_protection:
    message: "Test message"
"#;

        let rulebook: Rulebook =
            serde_yaml_ng::from_str(yaml_content).expect("Failed to parse YAML");

        let signals = rulebook.builtins.generate_signals();

        // These global builtins no longer generate signals - they use builtin_config
        // This is intentional to avoid spawning shell processes for static values
        assert!(
            signals.is_empty(),
            "Global builtins should not generate signals"
        );

        // The configuration is instead injected directly via builtin_config
        // during the gather_signals phase in the engine
    }
}
