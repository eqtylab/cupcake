use serde_json::json;
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_inject_context_basic_from_command() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a simple policy file directly in temp_dir
    let policy_content = r#"
SessionStart:
  "*":
    - name: test-inject
      description: Test injection
      conditions: []
      action:
        type: inject_context
        from_command:
          spec:
            mode: array
            command: ["echo", "Hello from command"]
          on_failure: continue
        use_stdout: true
"#;

    let config_path = temp_dir.path().join("test.yaml");
    fs::write(&config_path, policy_content).unwrap();

    let event_json = json!({
        "hook_event_name": "SessionStart",
        "session_id": "test-session",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "source": "startup"
    });

    let output = Command::new(env!("CARGO_BIN_EXE_cupcake"))
        .arg("run")
        .arg("--event")
        .arg("SessionStart")
        .arg("--config")
        .arg(config_path.to_str().unwrap())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap()
        .with_stdin(|stdin| {
            stdin.write_all(event_json.to_string().as_bytes()).unwrap();
        })
        .wait_with_output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    println!("Exit code: {:?}", output.status.code());
    println!("STDOUT: {}", stdout);
    println!("STDERR: {}", stderr);
    
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout.contains("Hello from command"));
}

// Helper trait for Command builder pattern
trait CommandExt {
    fn with_stdin<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut std::process::ChildStdin);
}

impl CommandExt for std::process::Child {
    fn with_stdin<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut std::process::ChildStdin),
    {
        if let Some(ref mut stdin) = self.stdin {
            f(stdin);
        }
        self
    }
}