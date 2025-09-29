//! Compiler module - Invokes OPA to create unified WASM module
//!
//! Implements the NEW_GUIDING_FINAL.md Hybrid Model compilation:
//! "Single entrypoint aggregation with cupcake.system.evaluate"

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, error, info};

use super::PolicyUnit;

/// Find the OPA binary, checking bundled location first
pub fn find_opa_binary() -> PathBuf {
    // 1. Check if OPA is bundled alongside cupcake binary
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            let bundled_opa = if cfg!(windows) {
                exe_dir.join("opa.exe")
            } else {
                exe_dir.join("opa")
            };

            if bundled_opa.exists() {
                debug!("Using bundled OPA at: {:?}", bundled_opa);
                return bundled_opa;
            }
        }
    }

    // 2. Check CUPCAKE_OPA_PATH environment variable
    if let Ok(opa_path) = std::env::var("CUPCAKE_OPA_PATH") {
        let path = PathBuf::from(opa_path);
        if path.exists() {
            debug!("Using OPA from CUPCAKE_OPA_PATH: {:?}", path);
            return path;
        }
    }

    // 3. Fall back to system PATH
    debug!("Using OPA from system PATH");
    if cfg!(windows) {
        PathBuf::from("opa.exe")
    } else {
        PathBuf::from("opa")
    }
}

/// Compile all policies into a single unified WASM module using OPA
pub async fn compile_policies(policies: &[PolicyUnit]) -> Result<Vec<u8>> {
    compile_policies_with_namespace(policies, "cupcake.system").await
}

/// Compile policies with a specific namespace for the entrypoint
pub async fn compile_policies_with_namespace(
    policies: &[PolicyUnit],
    namespace: &str,
) -> Result<Vec<u8>> {
    if policies.is_empty() {
        bail!("No policies to compile");
    }

    info!(
        "Compiling {} policies into unified WASM module",
        policies.len()
    );

    // Create a temporary directory for the compilation with a unique ID
    // TODO: Consider using the `tempfile` crate for cleaner temp directory management
    // that automatically handles uniqueness and cleanup
    use std::sync::atomic::{AtomicU64, Ordering};
    static COMPILE_ID: AtomicU64 = AtomicU64::new(0);
    let compile_id = COMPILE_ID.fetch_add(1, Ordering::SeqCst);
    let temp_dir = std::env::temp_dir().join(format!(
        "cupcake-compile-{}-{}",
        std::process::id(),
        compile_id
    ));
    tokio::fs::create_dir_all(&temp_dir).await?;

    // Canonicalize to ensure we have an absolute path
    let temp_dir = tokio::fs::canonicalize(&temp_dir)
        .await
        .context("Failed to canonicalize temp directory path")?;

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

    for policy in policies.iter() {
        // Get the relative path from the policies root
        let relative_path = policy
            .path
            .strip_prefix(policies_root)
            .unwrap_or_else(|_| policy.path.file_name().unwrap().as_ref());

        let dest_path = temp_path.join(relative_path);

        // Create parent directories if needed (for system/ subdirectory)
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Read the policy content
        let mut content = tokio::fs::read_to_string(&policy.path)
            .await
            .context(format!("Failed to read policy {:?}", policy.path))?;

        // If this is for global namespace and the package needs transformation
        if namespace.starts_with("cupcake.global") && !content.contains("package cupcake.global") {
            // Transform package declarations to global namespace
            content = content
                .replace(
                    "package cupcake.policies",
                    "package cupcake.global.policies",
                )
                .replace("package cupcake.system", "package cupcake.global.system");
            debug!(
                "Transformed policy {} to global namespace",
                policy.package_name
            );
        }

        // Write the (possibly transformed) content to temp dir
        tokio::fs::write(&dest_path, &content)
            .await
            .context(format!("Failed to write policy {dest_path:?}"))?;

        debug!(
            "Policy {} content preview (first 200 chars): {}",
            policy.package_name,
            &content.chars().take(200).collect::<String>()
        );

        debug!(
            "Wrote policy {} to temp: {:?}",
            policy.package_name, dest_path
        );
    }

    // Build the OPA command for Hybrid Model
    // Single entrypoint: cupcake.system.evaluate
    let opa_path = find_opa_binary();
    debug!("Using OPA binary: {:?}", opa_path);
    let mut opa_cmd = Command::new(&opa_path);
    opa_cmd
        .arg("build")
        .arg("-t")
        .arg("wasm") // Target WASM
        .arg("-O")
        .arg("2"); // Optimization level 2

    // Add the single aggregation entrypoint for the Hybrid Model
    // This entrypoint collects all decision verbs across all policies
    let entrypoint = format!("{}/evaluate", namespace.replace('.', "/"));
    opa_cmd.arg("-e").arg(&entrypoint);
    debug!("Added single Hybrid Model entrypoint: {}", entrypoint);

    // Add all policy files
    // On Windows, canonicalize() produces UNC paths with \\?\ prefix
    // which OPA doesn't understand. We need to strip this prefix.
    let temp_path_str = temp_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to convert temp path to string"))?;

    // Strip Windows UNC path prefix (\\?\) if present
    // OPA can't handle UNC paths and strips them incorrectly
    eprintln!("[CUPCAKE DEBUG] Before strip: {:?}", temp_path_str);
    let temp_path_arg = if cfg!(windows) && temp_path_str.starts_with(r"\\?\") {
        eprintln!("[CUPCAKE DEBUG] Stripping UNC prefix");
        temp_path_str.trim_start_matches(r"\\?\").to_string()
    } else {
        eprintln!("[CUPCAKE DEBUG] No UNC prefix found or not Windows");
        temp_path_str.to_string()
    };

    eprintln!("[CUPCAKE DEBUG] After strip, passing to OPA: {:?}", temp_path_arg);
    debug!("Temp path for OPA: {:?}", temp_path_arg);
    opa_cmd.arg(&temp_path_arg);

    // Output to bundle.tar.gz in temp dir
    let bundle_path = temp_path.join("bundle.tar.gz");
    let bundle_path_str = bundle_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to convert bundle path to string"))?;

    // Strip Windows UNC path prefix for bundle output path as well
    let bundle_path_arg = if cfg!(windows) && bundle_path_str.starts_with(r"\\?\") {
        bundle_path_str.trim_start_matches(r"\\?\").to_string()
    } else {
        bundle_path_str.to_string()
    };

    debug!("Bundle path: {:?}", bundle_path_arg);
    opa_cmd.arg("-o").arg(&bundle_path_arg);

    info!("Executing OPA build command...");
    debug!("OPA command: {:?}", opa_cmd);

    // Execute the command - this MUST work or we fail
    let output = opa_cmd
        .output()
        .context("Failed to execute OPA command. Is the OPA CLI installed and in your PATH?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        error!(
            "OPA command failed: {:?}\nStderr: {}\nStdout: {}",
            opa_cmd, stderr, stdout
        );

        // Provide more context on Windows
        #[cfg(windows)]
        {
            error!("Windows-specific debugging:");
            error!("Exit code: {:?}", output.status.code());
            error!("OPA binary path used: {}", opa_path.display());

            // Try to check if OPA exists
            if !opa_path.exists() {
                error!("OPA binary does not exist at: {}", opa_path.display());
            } else {
                error!("OPA binary exists at: {}", opa_path.display());
            }
        }

        let error_msg = if !stderr.is_empty() {
            format!("stderr: {}", stderr)
        } else if !stdout.is_empty() {
            format!("stdout: {}", stdout)
        } else {
            format!("No output from OPA. Exit code: {:?}", output.status.code())
        };
        bail!("OPA compilation failed: {}", error_msg);
    }

    info!("OPA compilation successful");

    // Extract the WASM module from the bundle
    let wasm_bytes = extract_wasm_from_bundle(&bundle_path)
        .await
        .context("Failed to extract WASM from bundle")?;

    info!("Extracted WASM module: {} bytes", wasm_bytes.len());

    // Debug: Save WASM to temp file for inspection
    let debug_wasm_path = std::env::temp_dir().join(format!(
        "cupcake-debug-{}-{}.wasm",
        std::process::id(),
        compile_id
    ));
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
    let temp_dir = std::env::temp_dir().join(format!(
        "cupcake-extract-{}-{}",
        std::process::id(),
        extract_id
    ));
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
        bail!(
            "Failed to extract bundle: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Read the policy.wasm file
    let wasm_path = extract_path.join("policy.wasm");
    if !wasm_path.exists() {
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
