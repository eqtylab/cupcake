use cupcake_core::engine::rulebook::{Rulebook, SignalConfig};
use serde_json::{json, Value};

#[tokio::test]
async fn test_signal_json_parsing_string() {
    let mut rulebook = Rulebook::default();

    // Add a signal that outputs a JSON string
    rulebook.signals.insert(
        "test_string".to_string(),
        SignalConfig {
            command: r#"echo '"hello world"'"#.to_string(),
            timeout_seconds: 5,
        },
    );

    let result = rulebook.execute_signal("test_string").await.unwrap();

    // Should parse as JSON string
    assert_eq!(result, Value::String("hello world".to_string()));
}

#[tokio::test]
async fn test_signal_json_parsing_object() {
    let mut rulebook = Rulebook::default();

    // Add a signal that outputs a JSON object
    rulebook.signals.insert(
        "test_object".to_string(),
        SignalConfig {
            command: r#"echo '{"key": "value", "number": 42, "bool": true}'"#.to_string(),
            timeout_seconds: 5,
        },
    );

    let result = rulebook.execute_signal("test_object").await.unwrap();

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
    let mut rulebook = Rulebook::default();

    // Add a signal that outputs a JSON array
    rulebook.signals.insert(
        "test_array".to_string(),
        SignalConfig {
            command: r#"echo '["item1", "item2", 123]'"#.to_string(),
            timeout_seconds: 5,
        },
    );

    let result = rulebook.execute_signal("test_array").await.unwrap();

    // Should parse as JSON array
    let expected = json!(["item1", "item2", 123]);
    assert_eq!(result, expected);
}

#[tokio::test]
async fn test_signal_invalid_json_fallback() {
    let mut rulebook = Rulebook::default();

    // Add a signal that outputs invalid JSON
    rulebook.signals.insert(
        "test_invalid".to_string(),
        SignalConfig {
            command: r#"echo 'this is not valid JSON {'"#.to_string(),
            timeout_seconds: 5,
        },
    );

    let result = rulebook.execute_signal("test_invalid").await.unwrap();

    // Should fall back to string storage
    assert_eq!(
        result,
        Value::String("this is not valid JSON {".to_string())
    );
}

#[tokio::test]
async fn test_signal_empty_output() {
    let mut rulebook = Rulebook::default();

    // Add a signal that outputs nothing
    rulebook.signals.insert(
        "test_empty".to_string(),
        SignalConfig {
            command: r#"echo"#.to_string(), // Just echo with no args
            timeout_seconds: 5,
        },
    );

    let result = rulebook.execute_signal("test_empty").await.unwrap();

    // Should be empty string stored as JSON string
    assert_eq!(result, Value::String("".to_string()));
}

#[tokio::test]
async fn test_signal_whitespace_trimming() {
    let mut rulebook = Rulebook::default();

    // Add a signal that outputs JSON with whitespace
    rulebook.signals.insert(
        "test_whitespace".to_string(),
        SignalConfig {
            command: r#"echo '   "trimmed"   '"#.to_string(),
            timeout_seconds: 5,
        },
    );

    let result = rulebook.execute_signal("test_whitespace").await.unwrap();

    // Should parse as JSON string with whitespace trimmed
    assert_eq!(result, Value::String("trimmed".to_string()));
}

#[tokio::test]
async fn test_execute_signals_concurrent() {
    let mut rulebook = Rulebook::default();

    // Add multiple signals
    rulebook.signals.insert(
        "signal1".to_string(),
        SignalConfig {
            command: r#"echo '"value1"'"#.to_string(),
            timeout_seconds: 5,
        },
    );

    rulebook.signals.insert(
        "signal2".to_string(),
        SignalConfig {
            command: r#"echo '{"key": "value2"}'"#.to_string(),
            timeout_seconds: 5,
        },
    );

    rulebook.signals.insert(
        "signal3".to_string(),
        SignalConfig {
            command: r#"echo '[1, 2, 3]'"#.to_string(),
            timeout_seconds: 5,
        },
    );

    let signal_names = vec![
        "signal1".to_string(),
        "signal2".to_string(),
        "signal3".to_string(),
    ];

    let results = rulebook.execute_signals(&signal_names).await.unwrap();

    assert_eq!(results.len(), 3);
    assert_eq!(results["signal1"], Value::String("value1".to_string()));
    assert_eq!(results["signal2"], json!({"key": "value2"}));
    assert_eq!(results["signal3"], json!([1, 2, 3]));
}

#[tokio::test]
async fn test_execute_signals_with_failures() {
    let mut rulebook = Rulebook::default();

    // Add a signal that will succeed
    rulebook.signals.insert(
        "good_signal".to_string(),
        SignalConfig {
            command: r#"echo '"success"'"#.to_string(),
            timeout_seconds: 5,
        },
    );

    // Add a signal that will fail
    rulebook.signals.insert(
        "bad_signal".to_string(),
        SignalConfig {
            command: r#"exit 1"#.to_string(),
            timeout_seconds: 5,
        },
    );

    let signal_names = vec!["good_signal".to_string(), "bad_signal".to_string()];

    let results = rulebook.execute_signals(&signal_names).await.unwrap();

    // Both signals should be present - failed signals return error details for validation
    assert_eq!(results.len(), 2);

    // Good signal returns its output
    assert_eq!(results["good_signal"], Value::String("success".to_string()));

    // Bad signal returns structured error information
    assert!(results.contains_key("bad_signal"));
    let bad_signal_result = &results["bad_signal"];
    assert_eq!(bad_signal_result["exit_code"], 1);
    assert_eq!(bad_signal_result["success"], false);
    assert_eq!(bad_signal_result["output"], ""); // exit 1 produces no output
    assert_eq!(bad_signal_result["error"], ""); // exit 1 produces no error message
}

#[tokio::test]
async fn test_complex_structured_signal() {
    let mut rulebook = Rulebook::default();

    // Add a signal that outputs complex nested JSON (like our test_status example)
    rulebook.signals.insert(
        "complex_signal".to_string(),
        SignalConfig {
            command: r#"echo '{
            "passing": false,
            "coverage": 87.5,
            "duration": 12.3,
            "failed_tests": ["test1", "test2"],
            "metadata": {
                "branch": "main",
                "commit": "abc123"
            }
        }'"#
            .to_string(),
            timeout_seconds: 5,
        },
    );

    let result = rulebook.execute_signal("complex_signal").await.unwrap();

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
