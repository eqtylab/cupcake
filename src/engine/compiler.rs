//! Compiler module - Invokes OPA to create unified WASM module
//! 
//! Implements the NEW_GUIDING_FINAL.md Hybrid Model compilation:
//! "Single entrypoint aggregation with cupcake.system.evaluate"

use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;
use tracing::{debug, error, info};

use super::PolicyUnit;

/// Compile all policies into a single unified WASM module using OPA
pub async fn compile_policies(policies: &[PolicyUnit]) -> Result<Vec<u8>> {
    if policies.is_empty() {
        bail!("No policies to compile");
    }
    
    info!("Compiling {} policies into unified WASM module", policies.len());
    
    // Create a temporary directory for the compilation with a unique ID
    // TODO: Consider using the `tempfile` crate for cleaner temp directory management
    // that automatically handles uniqueness and cleanup
    use std::sync::atomic::{AtomicU64, Ordering};
    static COMPILE_ID: AtomicU64 = AtomicU64::new(0);
    let compile_id = COMPILE_ID.fetch_add(1, Ordering::SeqCst);
    let temp_dir = std::env::temp_dir().join(format!("cupcake-compile-{}-{}", std::process::id(), compile_id));
    tokio::fs::create_dir_all(&temp_dir).await?;
    let temp_path = temp_dir.as_path();
    debug!("Using temp directory: {:?}", temp_dir);
    
    // Write all policies to the temp directory, preserving directory structure
    debug!("Copying {} policies to temp dir", policies.len());
    
    // Find the common policies directory root
    let policies_root = if !policies.is_empty() {
        // Assume all policies are under a common "policies" directory
        let first_policy_path = &policies[0].path;
        let mut current = first_policy_path.parent();
        while let Some(parent) = current {
            if parent.file_name() == Some(std::ffi::OsStr::new("policies")) {
                break;
            }
            current = parent.parent();
        }
        current.unwrap_or_else(|| first_policy_path.parent().unwrap())
    } else {
        bail!("No policies to determine root from");
    };
    
    debug!("Policies root: {:?}", policies_root);
    
    for (_i, policy) in policies.iter().enumerate() {
        // Get the relative path from the policies root
        let relative_path = policy.path.strip_prefix(policies_root)
            .unwrap_or_else(|_| policy.path.file_name().unwrap().as_ref());
        
        let dest_path = temp_path.join(relative_path);
        
        // Create parent directories if needed (for system/ subdirectory)
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        // Copy the policy file to temp dir
        tokio::fs::copy(&policy.path, &dest_path)
            .await
            .context(format!("Failed to copy policy {:?}", policy.path))?;
        
        // Debug: Read and print the copied file to verify content
        let copied_content = tokio::fs::read_to_string(&dest_path).await?;
        debug!("Policy {} content preview (first 200 chars): {}", 
            policy.package_name, 
            &copied_content.chars().take(200).collect::<String>());
        
        debug!("Copied policy {} to temp: {:?}", policy.package_name, dest_path);
    }
    
    // Build the OPA command for Hybrid Model
    // Single entrypoint: cupcake.system.evaluate
    let mut opa_cmd = Command::new("opa");
    opa_cmd.arg("build")
        .arg("-t").arg("wasm")   // Target WASM
        .arg("-O").arg("2");      // Optimization level 2
    
    // Add the single aggregation entrypoint for the Hybrid Model
    // This entrypoint collects all decision verbs across all policies
    opa_cmd.arg("-e").arg("cupcake/system/evaluate");
    debug!("Added single Hybrid Model entrypoint: cupcake/system/evaluate");
    
    // Add all policy files
    opa_cmd.arg(temp_path);
    
    // Output to bundle.tar.gz in temp dir
    let bundle_path = temp_path.join("bundle.tar.gz");
    opa_cmd.arg("-o").arg(&bundle_path);
    
    info!("Executing OPA build command...");
    debug!("OPA command: {:?}", opa_cmd);
    
    // Execute the command - this MUST work or we fail
    let output = opa_cmd.output()
        .context("Failed to execute OPA command. Is the OPA CLI installed and in your PATH?")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        error!("OPA command failed: {:?}\nStderr: {}\nStdout: {}", opa_cmd, stderr, stdout);
        bail!("OPA compilation failed: {}", stderr);
    }
    
    info!("OPA compilation successful");
    
    // Extract the WASM module from the bundle
    let wasm_bytes = extract_wasm_from_bundle(&bundle_path)
        .await
        .context("Failed to extract WASM from bundle")?;
    
    info!("Extracted WASM module: {} bytes", wasm_bytes.len());
    
    // Debug: Save WASM to temp file for inspection
    let debug_wasm_path = std::env::temp_dir().join(format!("cupcake-debug-{}-{}.wasm", std::process::id(), compile_id));
    tokio::fs::write(&debug_wasm_path, &wasm_bytes).await?;
    debug!("Saved WASM to {:?} for debugging", debug_wasm_path);
    
    Ok(wasm_bytes)
}

/// Extract the policy.wasm file from the OPA bundle
async fn extract_wasm_from_bundle(bundle_path: &Path) -> Result<Vec<u8>> {
    // OPA creates a tar.gz bundle, we need to extract policy.wasm from it
    
    let bundle_bytes = tokio::fs::read(bundle_path)
        .await
        .context("Failed to read bundle file")?;
    
    // Use tar command to extract (simpler than adding tar crate dependency)
    // TODO: Consider using the `tempfile` crate here as well for automatic cleanup
    use std::sync::atomic::{AtomicU64, Ordering};
    static EXTRACT_ID: AtomicU64 = AtomicU64::new(0);
    let extract_id = EXTRACT_ID.fetch_add(1, Ordering::SeqCst);
    let temp_dir = std::env::temp_dir().join(format!("cupcake-extract-{}-{}", std::process::id(), extract_id));
    std::fs::create_dir_all(&temp_dir)?;
    let extract_path = temp_dir.as_path();
    
    // Write bundle to temp location
    let temp_bundle = extract_path.join("bundle.tar.gz");
    tokio::fs::write(&temp_bundle, &bundle_bytes).await?;
    
    // Extract using tar command
    let output = Command::new("tar")
        .arg("-xzf")
        .arg(&temp_bundle)
        .arg("-C")
        .arg(extract_path)
        .output()
        .context("Failed to extract tar bundle")?;
    
    if !output.status.success() {
        bail!("Failed to extract bundle: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    // Read the policy.wasm file
    let wasm_path = extract_path.join("policy.wasm");
    if !wasm_path.exists() {
        // Sometimes it's at /policy.wasm in the tar
        let alt_wasm_path = extract_path.join("/policy.wasm");
        if alt_wasm_path.exists() {
            return Ok(tokio::fs::read(alt_wasm_path).await?);
        }
        bail!("No policy.wasm found in OPA bundle");
    }
    
    Ok(tokio::fs::read(wasm_path).await?)
}



// Aligns with NEW_GUIDING_FINAL.md:
// - Compiles all discovered policies into a SINGLE unified WASM module
// - Uses OPA build with optimization (-O 2) for performance
// - Exports single aggregation entrypoint: cupcake/system/evaluate
// - Handles compilation failures gracefully with clear errors
// - Foundation for the Hybrid Model's sub-millisecond performance target