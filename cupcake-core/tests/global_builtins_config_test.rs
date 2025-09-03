#[cfg(test)]
mod tests {
    use cupcake_core::engine::guidebook::Guidebook;

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

        // Parse the Guidebook
        let guidebook: Guidebook = serde_yaml_ng::from_str(yaml_content)
            .expect("Failed to parse YAML");
        
        let config = guidebook.builtins;
        
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

        let guidebook: Guidebook = serde_yaml_ng::from_str(yaml_content)
            .expect("Failed to parse YAML");
        
        let config = guidebook.builtins;
        
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
        assert!(exec_prot.enabled); // Should default to true
        assert_eq!(exec_prot.message, "Direct execution of Cupcake binary is not permitted");
        
        // Test enabled_builtins - only system_protection and cupcake_exec_protection
        let enabled = config.enabled_builtins();
        assert!(enabled.contains(&"system_protection".to_string()));
        assert!(!enabled.contains(&"sensitive_data_protection".to_string()));
        assert!(enabled.contains(&"cupcake_exec_protection".to_string()));
    }
    
    #[test]
    fn test_global_builtins_signal_generation() {
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

        let guidebook: Guidebook = serde_yaml_ng::from_str(yaml_content)
            .expect("Failed to parse YAML");
        
        let signals = guidebook.builtins.generate_signals();
        
        // Check system protection signals
        assert!(signals.contains_key("__builtin_system_protection_paths"));
        assert!(signals.contains_key("__builtin_system_protection_message"));
        
        // Check sensitive data signals
        assert!(signals.contains_key("__builtin_sensitive_data_patterns"));
        
        // Check cupcake exec signals
        assert!(signals.contains_key("__builtin_cupcake_exec_message"));
    }
}