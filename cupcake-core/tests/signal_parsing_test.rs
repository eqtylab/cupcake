use cupcake_core::engine::guidebook::{Guidebook, SignalConfig};
use serde_json::{json, Value};
use tokio;

#[tokio::test]
async fn test_signal_json_parsing_string() {
    let mut guidebook = Guidebook::default();
    
    // Add a signal that outputs a JSON string
    guidebook.signals.insert("test_string".to_string(), SignalConfig {
        command: r#"echo '"hello world"'"#.to_string(),
        timeout_seconds: 5,
    });
    
    let result = guidebook.execute_signal("test_string").await.unwrap();
    
    // Should parse as JSON string
    assert_eq!(result, Value::String("hello world".to_string()));
}

#[tokio::test]
async fn test_signal_json_parsing_object() {
    let mut guidebook = Guidebook::default();
    
    // Add a signal that outputs a JSON object
    guidebook.signals.insert("test_object".to_string(), SignalConfig {
        command: r#"echo '{"key": "value", "number": 42, "bool": true}'"#.to_string(),
        timeout_seconds: 5,
    });
    
    let result = guidebook.execute_signal("test_object").await.unwrap();
    
    // Should parse as JSON object
    let expected = json!({
        "key": "value",
        "number": 42,
        "bool": true
    });
    assert_eq!(result, expected);
}

#[tokio::test]
async fn test_signal_json_parsing_array() {
    let mut guidebook = Guidebook::default();
    
    // Add a signal that outputs a JSON array
    guidebook.signals.insert("test_array".to_string(), SignalConfig {
        command: r#"echo '["item1", "item2", 123]'"#.to_string(),
        timeout_seconds: 5,
    });
    
    let result = guidebook.execute_signal("test_array").await.unwrap();
    
    // Should parse as JSON array
    let expected = json!(["item1", "item2", 123]);
    assert_eq!(result, expected);
}

#[tokio::test]
async fn test_signal_invalid_json_fallback() {
    let mut guidebook = Guidebook::default();
    
    // Add a signal that outputs invalid JSON
    guidebook.signals.insert("test_invalid".to_string(), SignalConfig {
        command: r#"echo 'this is not valid JSON {'"#.to_string(),
        timeout_seconds: 5,
    });
    
    let result = guidebook.execute_signal("test_invalid").await.unwrap();
    
    // Should fall back to string storage
    assert_eq!(result, Value::String("this is not valid JSON {".to_string()));
}

#[tokio::test]
async fn test_signal_empty_output() {
    let mut guidebook = Guidebook::default();
    
    // Add a signal that outputs nothing
    guidebook.signals.insert("test_empty".to_string(), SignalConfig {
        command: r#"echo"#.to_string(),  // Just echo with no args
        timeout_seconds: 5,
    });
    
    let result = guidebook.execute_signal("test_empty").await.unwrap();
    
    // Should be empty string stored as JSON string
    assert_eq!(result, Value::String("".to_string()));
}

#[tokio::test]
async fn test_signal_whitespace_trimming() {
    let mut guidebook = Guidebook::default();
    
    // Add a signal that outputs JSON with whitespace
    guidebook.signals.insert("test_whitespace".to_string(), SignalConfig {
        command: r#"echo '   "trimmed"   '"#.to_string(),
        timeout_seconds: 5,
    });
    
    let result = guidebook.execute_signal("test_whitespace").await.unwrap();
    
    // Should parse as JSON string with whitespace trimmed
    assert_eq!(result, Value::String("trimmed".to_string()));
}

#[tokio::test]
async fn test_execute_signals_concurrent() {
    let mut guidebook = Guidebook::default();
    
    // Add multiple signals
    guidebook.signals.insert("signal1".to_string(), SignalConfig {
        command: r#"echo '"value1"'"#.to_string(),
        timeout_seconds: 5,
    });
    
    guidebook.signals.insert("signal2".to_string(), SignalConfig {
        command: r#"echo '{"key": "value2"}'"#.to_string(),
        timeout_seconds: 5,
    });
    
    guidebook.signals.insert("signal3".to_string(), SignalConfig {
        command: r#"echo '[1, 2, 3]'"#.to_string(),
        timeout_seconds: 5,
    });
    
    let signal_names = vec![
        "signal1".to_string(),
        "signal2".to_string(), 
        "signal3".to_string()
    ];
    
    let results = guidebook.execute_signals(&signal_names).await.unwrap();
    
    assert_eq!(results.len(), 3);
    assert_eq!(results["signal1"], Value::String("value1".to_string()));
    assert_eq!(results["signal2"], json!({"key": "value2"}));
    assert_eq!(results["signal3"], json!([1, 2, 3]));
}

#[tokio::test]
async fn test_execute_signals_with_failures() {
    let mut guidebook = Guidebook::default();
    
    // Add a signal that will succeed
    guidebook.signals.insert("good_signal".to_string(), SignalConfig {
        command: r#"echo '"success"'"#.to_string(),
        timeout_seconds: 5,
    });
    
    // Add a signal that will fail
    guidebook.signals.insert("bad_signal".to_string(), SignalConfig {
        command: r#"exit 1"#.to_string(),
        timeout_seconds: 5,
    });
    
    let signal_names = vec![
        "good_signal".to_string(),
        "bad_signal".to_string()
    ];
    
    let results = guidebook.execute_signals(&signal_names).await.unwrap();
    
    // Both signals should be present - failed signals return error details for validation
    assert_eq!(results.len(), 2);
    
    // Good signal returns its output
    assert_eq!(results["good_signal"], Value::String("success".to_string()));
    
    // Bad signal returns structured error information
    assert!(results.contains_key("bad_signal"));
    let bad_signal_result = &results["bad_signal"];
    assert_eq!(bad_signal_result["exit_code"], 1);
    assert_eq!(bad_signal_result["success"], false);
    assert_eq!(bad_signal_result["output"], "");  // exit 1 produces no output
    assert_eq!(bad_signal_result["error"], "");   // exit 1 produces no error message
}

#[tokio::test]
async fn test_complex_structured_signal() {
    let mut guidebook = Guidebook::default();
    
    // Add a signal that outputs complex nested JSON (like our test_status example)
    guidebook.signals.insert("complex_signal".to_string(), SignalConfig {
        command: r#"echo '{
            "passing": false,
            "coverage": 87.5,
            "duration": 12.3,
            "failed_tests": ["test1", "test2"],
            "metadata": {
                "branch": "main",
                "commit": "abc123"
            }
        }'"#.to_string(),
        timeout_seconds: 5,
    });
    
    let result = guidebook.execute_signal("complex_signal").await.unwrap();
    
    let expected = json!({
        "passing": false,
        "coverage": 87.5,
        "duration": 12.3,
        "failed_tests": ["test1", "test2"],
        "metadata": {
            "branch": "main",
            "commit": "abc123"
        }
    });
    
    assert_eq!(result, expected);
}