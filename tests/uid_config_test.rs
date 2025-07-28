//! Tests for configurable UID functionality
//! 
//! This test validates that the sandbox_uid setting properly controls
//! the UID drop for shell commands.

use cupcake::config::types::Settings;

#[test]
fn test_uid_configuration() {
    // Test numeric UID
    let yaml = r#"
debug_mode: false
allow_shell: true
sandbox_uid: "65534"
"#;
    
    let settings: Settings = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(settings.sandbox_uid, Some("65534".to_string()));
    
    // Test username
    let yaml_username = r#"
debug_mode: false
allow_shell: true
sandbox_uid: "nobody"
"#;
    
    let settings2: Settings = serde_yaml_ng::from_str(yaml_username).unwrap();
    assert_eq!(settings2.sandbox_uid, Some("nobody".to_string()));
    
    // Test default (no UID drop)
    let yaml_no_uid = r#"
debug_mode: false
allow_shell: true
"#;
    
    let settings3: Settings = serde_yaml_ng::from_str(yaml_no_uid).unwrap();
    assert_eq!(settings3.sandbox_uid, None);
}

#[test]
fn test_uid_default() {
    let settings = Settings::default();
    assert!(settings.sandbox_uid.is_none());
}

// Note: Actual UID drop testing requires root privileges and would be done
// in integration tests or manual testing. Here we just verify configuration.