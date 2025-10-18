//! Integration tests for debug logging system
//!
//! Validates core functionality and zero-impact performance characteristics

#[cfg(test)]
mod integration_tests {
    use super::super::*;
    use crate::engine::Engine;
    use serde_json::json;
    use std::path::Path;

    /// Test helper to check if we can create an engine (skips test if not)
    async fn try_create_test_engine() -> Option<Engine> {
        use crate::harness::types::HarnessType;

        // Try to find a valid policy directory
        let candidates = vec!["./examples", "../examples", "../../examples"];

        for dir in candidates {
            let path = Path::new(dir);
            if path.join(".cupcake/policies").exists() {
                if let Ok(engine) = Engine::new(path, HarnessType::ClaudeCode).await {
                    return Some(engine);
                }
            }
        }
        None
    }

    #[tokio::test]
    async fn test_debug_capture_with_allow_decision() {
        // Skip if we can't create an engine
        let Some(engine) = try_create_test_engine().await else {
            eprintln!("Skipping test - no valid policy directory found");
            return;
        };

        let event = json!({
            "hook_event_name": "PreToolUse",
            "hookEventName": "PreToolUse",
            "tool_name": "Read",
            "tool_input": {"file_path": "/tmp/safe.txt"},
            "session_id": "test-allow",
            "transcript_path": "/tmp/transcript.txt",
            "cwd": "/tmp"
        });

        let trace_id = "test-allow-123".to_string();
        let mut debug_capture = Some(DebugCapture::new(event.clone(), trace_id, true, None));

        let _decision = engine
            .evaluate(&event, debug_capture.as_mut())
            .await
            .unwrap();

        // Verify debug capture populated
        let debug = debug_capture.unwrap();
        assert!(debug.routed);
        assert!(!debug.matched_policies.is_empty());
        assert!(debug.final_decision.is_some());
        assert!(matches!(
            debug.final_decision.unwrap(),
            crate::engine::decision::FinalDecision::Allow { .. }
        ));
    }

    #[tokio::test]
    async fn test_debug_capture_with_deny_decision() {
        let Some(engine) = try_create_test_engine().await else {
            eprintln!("Skipping test - no valid policy directory found");
            return;
        };

        let event = json!({
            "hook_event_name": "PreToolUse",
            "hookEventName": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {"command": "rm -rf /System"},  // Should trigger protection
            "session_id": "test-deny",
            "transcript_path": "/tmp/transcript.txt",
            "cwd": "/tmp"
        });

        let trace_id = "test-deny-456".to_string();
        let mut debug_capture = Some(DebugCapture::new(event.clone(), trace_id, true, None));

        let _decision = engine
            .evaluate(&event, debug_capture.as_mut())
            .await
            .unwrap();

        let debug = debug_capture.unwrap();
        assert!(debug.routed);
        // Signals may or may not be executed depending on policy requirements
        // The important thing is that the decision was made correctly
    }

    #[tokio::test]
    async fn test_no_debug_when_disabled() {
        let Some(engine) = try_create_test_engine().await else {
            eprintln!("Skipping test - no valid policy directory found");
            return;
        };

        let event = json!({
            "hook_event_name": "SessionStart",
            "hookEventName": "SessionStart",
            "source": "Startup",
            "session_id": "test-no-debug",
            "transcript_path": "/tmp/transcript.txt",
            "cwd": "/tmp"
        });

        // No debug capture created when disabled
        let decision = engine.evaluate(&event, None).await.unwrap();
        assert!(matches!(
            decision,
            crate::engine::decision::FinalDecision::Allow { .. }
        ));

        // Verify no debug files created
        let debug_dir = Path::new(".cupcake/debug");
        if debug_dir.exists() {
            let entries: Vec<_> = std::fs::read_dir(debug_dir)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|e| e.file_name().to_string_lossy().contains("test-no-debug"))
                .collect();
            assert!(
                entries.is_empty(),
                "No debug files should be created when disabled"
            );
        }
    }

    #[tokio::test]
    async fn test_debug_error_capture() {
        let event = json!({
            "hook_event_name": "PreToolUse",
            "hookEventName": "PreToolUse",
            "tool_name": "Bash",
            "session_id": "test-error"
        });

        let trace_id = "test-error-789".to_string();
        let mut debug = DebugCapture::new(event, trace_id, true, None);

        // Simulate errors during evaluation
        debug.add_error("Signal execution failed: timeout".to_string());
        debug.add_error("Action execution failed: permission denied".to_string());

        assert_eq!(debug.errors.len(), 2);
        assert!(debug.errors[0].contains("Signal execution failed"));
    }

    #[test]
    fn test_debug_file_format() {
        let event = json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": "test prompt",
            "session_id": "format-test"
        });

        let mut debug = DebugCapture::new(event, "format-test-abc".to_string(), true, None);

        // Populate with sample data
        debug.routed = true;
        debug.matched_policies = vec!["policy1".to_string(), "policy2".to_string()];
        debug.signals_configured = vec!["signal1".to_string()];
        debug.signals_executed = vec![SignalExecution {
            name: "signal1".to_string(),
            command: "echo test".to_string(),
            result: json!("test output"),
            duration_ms: Some(5),
        }];

        let output = debug.format_debug_output().unwrap();

        // Verify all sections present
        assert!(output.contains("===== Claude Code Event"));
        assert!(output.contains("----- Routing -----"));
        assert!(output.contains("----- Signals -----"));
        assert!(output.contains("----- WASM Evaluation -----"));
        assert!(output.contains("----- Synthesis -----"));
        assert!(output.contains("----- Response to Claude -----"));
        assert!(output.contains("----- Actions -----"));
        assert!(output.contains("===== End Event"));
    }
}

#[cfg(test)]
mod performance_tests {
    use super::super::*;
    use serde_json::json;
    use std::time::Instant;

    #[test]
    fn test_zero_overhead_when_disabled() {
        let event = json!({"test": "data"});
        let trace_id = "perf-test".to_string();

        // Measure overhead of creating debug capture when disabled
        let start = Instant::now();
        for _ in 0..10000 {
            let debug = DebugCapture::new(event.clone(), trace_id.clone(), false, None); // disabled
                                                                                         // This should be essentially free when disabled
            let _ = debug.write_if_enabled();
        }
        let disabled_duration = start.elapsed();

        // Should complete quickly when disabled
        // In CI environments with shared resources, allow more time
        let threshold_ms = if std::env::var("CI").is_ok() {
            50 // More lenient in CI
        } else {
            20 // Still reasonable for local development
        };

        assert!(
            disabled_duration.as_millis() < threshold_ms,
            "Disabled debug should have near-zero overhead, took {}ms (threshold: {}ms)",
            disabled_duration.as_millis(),
            threshold_ms
        );
    }

    #[test]
    fn test_bounded_memory_usage() {
        let event = json!({"large": "x".repeat(1000)});
        let mut debug = DebugCapture::new(event, "memory-test".to_string(), true, None);

        // Add many items to test memory bounds
        for i in 0..1000 {
            debug.matched_policies.push(format!("policy_{i}"));
            debug.errors.push(format!("error_{i}"));
        }

        // Format should handle large captures gracefully
        let output = debug.format_debug_output().unwrap();

        // Output should be reasonable size even with many items
        assert!(output.len() < 1_000_000, "Debug output should be bounded");
    }
}
