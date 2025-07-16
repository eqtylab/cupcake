//! Audit logging module for command execution
//! 
//! This module provides the AuditSink trait and implementations for
//! logging command execution audit records to various destinations.

use crate::Result;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::fs::{OpenOptions, create_dir_all};
use tokio::io::AsyncWriteExt;
use chrono::Local;

/// Trait for audit log sinks
pub trait AuditSink: Send + Sync {
    /// Write an audit record
    async fn write(&self, record: &Value) -> Result<()>;
    
    /// Flush any buffered records
    async fn flush(&self) -> Result<()> {
        Ok(()) // Default no-op
    }
}

/// Stdout audit sink - writes JSON records to stdout
pub struct StdoutSink;

impl AuditSink for StdoutSink {
    async fn write(&self, record: &Value) -> Result<()> {
        println!("{}", record);
        Ok(())
    }
}

/// File-based audit sink with daily rotation
pub struct FileSink {
    base_path: PathBuf,
    current_file: Mutex<Option<PathBuf>>,
    current_date: Mutex<Option<String>>,
}

impl FileSink {
    /// Create a new file sink with the given base path
    /// 
    /// Files will be created as `{base_path}/exec-YYYYMMDD.jsonl`
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            current_file: Mutex::new(None),
            current_date: Mutex::new(None),
        }
    }
    
    /// Get the audit file path for the current date
    fn get_file_path(&self) -> PathBuf {
        let date = Local::now().format("%Y%m%d").to_string();
        self.base_path.join(format!("exec-{}.jsonl", date))
    }
    
    /// Check if we need to rotate to a new file
    fn needs_rotation(&self) -> bool {
        let current_date = Local::now().format("%Y%m%d").to_string();
        let locked_date = self.current_date.lock().unwrap();
        match &*locked_date {
            Some(date) => date != &current_date,
            None => true,
        }
    }
}

impl AuditSink for FileSink {
    async fn write(&self, record: &Value) -> Result<()> {
        // Check if we need to rotate files
        let file_path = if self.needs_rotation() {
            let new_date = Local::now().format("%Y%m%d").to_string();
            let new_path = self.get_file_path();
            *self.current_date.lock().unwrap() = Some(new_date);
            *self.current_file.lock().unwrap() = Some(new_path.clone());
            new_path
        } else {
            self.current_file.lock().unwrap().as_ref().unwrap().clone()
        };
        
        // Ensure directory exists
        if let Some(parent) = file_path.parent() {
            create_dir_all(parent).await
                .map_err(|e| crate::CupcakeError::Io(e))?;
        }
        
        // Append to file atomically
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)
            .await
            .map_err(|e| crate::CupcakeError::Io(e))?;
        
        // Write JSONL record
        let mut line = serde_json::to_string(record)
            .map_err(|e| crate::CupcakeError::Config(
                format!("Failed to serialize audit record: {}", e)
            ))?;
        line.push('\n');
        
        file.write_all(line.as_bytes()).await
            .map_err(|e| crate::CupcakeError::Io(e))?;
        
        file.flush().await
            .map_err(|e| crate::CupcakeError::Io(e))?;
        
        Ok(())
    }
}

/// Enum wrapper for audit sinks to enable dynamic dispatch with async methods
pub enum AuditSinkImpl {
    Stdout(StdoutSink),
    File(FileSink),
}

impl AuditSinkImpl {
    pub async fn write(&self, record: &Value) -> Result<()> {
        match self {
            AuditSinkImpl::Stdout(sink) => sink.write(record).await,
            AuditSinkImpl::File(sink) => sink.write(record).await,
        }
    }
    
    pub async fn flush(&self) -> Result<()> {
        match self {
            AuditSinkImpl::Stdout(sink) => sink.flush().await,
            AuditSinkImpl::File(sink) => sink.flush().await,
        }
    }
}

/// Factory function to create default audit sink
pub fn create_default_sink() -> AuditSinkImpl {
    // Check if audit directory is configured
    if let Ok(home) = std::env::var("HOME") {
        let audit_path = PathBuf::from(home).join(".cupcake").join("audit");
        AuditSinkImpl::File(FileSink::new(audit_path))
    } else {
        // Fallback to stdout if HOME not available
        AuditSinkImpl::Stdout(StdoutSink)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_stdout_sink() {
        let sink = StdoutSink;
        let record = serde_json::json!({
            "test": true,
            "timestamp": "2025-07-15T10:00:00Z"
        });
        
        // Should not panic
        sink.write(&record).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_file_sink_creation() {
        let temp_dir = tempdir().unwrap();
        let sink = FileSink::new(temp_dir.path().to_path_buf());
        
        let record = serde_json::json!({
            "test": true,
            "timestamp": "2025-07-15T10:00:00Z"
        });
        
        sink.write(&record).await.unwrap();
        
        // Check file was created
        let files: Vec<_> = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        
        assert_eq!(files.len(), 1);
        assert!(files[0].file_name().to_string_lossy().starts_with("exec-"));
        assert!(files[0].file_name().to_string_lossy().ends_with(".jsonl"));
        
        // Check content
        let content = std::fs::read_to_string(files[0].path()).unwrap();
        assert!(content.contains("\"test\":true"));
    }
    
    #[tokio::test]
    async fn test_file_sink_append() {
        let temp_dir = tempdir().unwrap();
        let sink = FileSink::new(temp_dir.path().to_path_buf());
        
        // Write multiple records
        for i in 0..3 {
            let record = serde_json::json!({
                "index": i,
                "timestamp": "2025-07-15T10:00:00Z"
            });
            sink.write(&record).await.unwrap();
        }
        
        // Check all records were written
        let files: Vec<_> = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        
        assert_eq!(files.len(), 1);
        
        let content = std::fs::read_to_string(files[0].path()).unwrap();
        let lines: Vec<_> = content.lines().collect();
        assert_eq!(lines.len(), 3);
        
        // Verify each line is valid JSON
        for (i, line) in lines.iter().enumerate() {
            let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
            assert_eq!(parsed["index"], i);
        }
    }
}