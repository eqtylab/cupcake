//! Compiler module - Invokes OPA to compile policies into a unified WASM module.
//!
//! Uses `cupcake.system.evaluate` as the single aggregation entrypoint.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use tracing::{debug, error, info};

use super::PolicyUnit;

/// Find the OPA binary with optional CLI override
///
/// # Resolution Order
///
/// 1. **CLI override** (if provided) - validated for existence and executability
/// 2. **Bundled OPA** alongside cupcake binary - checked for existence
/// 3. **System PATH** - returned as command name for OS resolution
///
/// # Validation Approach
///
/// Different discovery methods use appropriate validation strategies:
///
/// - **CLI override**: Validated early (user input should fail fast with clear errors)
/// - **Bundled OPA**: Existence check only (known specific location)
/// - **System PATH**: No pre-validation (follows standard Rust practice)
///
/// The PATH fallback returns a command name ("opa" or "opa.exe") that the OS
/// will resolve at execution time. If OPA is not found in PATH, the execution
/// will fail with a helpful error message (see `compile_policies_with_namespace`).
///
/// This approach avoids TOCTOU (time-of-check-time-of-use) issues and matches
/// the pattern used by cargo, rustup, and other Rust ecosystem tools.
///
/// # Returns
///
/// Returns `Ok(PathBuf)` with either an absolute path to a validated OPA binary, or a
/// command name ("opa"/"opa.exe") for PATH resolution. See "Resolution Order" for details.
///
/// # Errors
///
/// Returns `Err` only when CLI override validation fails. PATH resolution errors are
/// deferred to execution time for better error context and to avoid false negatives.
pub fn find_opa_binary(cli_override: Option<PathBuf>) -> Result<PathBuf> {
    // 1. Check CLI override
    if let Some(opa_path) = cli_override {
        // Validate the path
        if !opa_path.exists() {
            bail!("OPA path does not exist: {}", opa_path.display());
        }

        if !opa_path.is_file() {
            bail!("OPA path must be a file: {}", opa_path.display());
        }

        // Check if file is executable (Unix-like systems only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata =
                std::fs::metadata(&opa_path).context("Failed to read OPA file metadata")?;
            let permissions = metadata.permissions();
            if permissions.mode() & 0o111 == 0 {
                bail!("OPA path is not executable: {}", opa_path.display());
            }
        }

        debug!("Using CLI --opa-path override: {:?}", opa_path);
        return Ok(opa_path);
    }

    // 2. Check if OPA is bundled alongside cupcake binary
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            let bundled_opa = if cfg!(windows) {
                exe_dir.join("opa.exe")
            } else {
                exe_dir.join("opa")
            };

            if bundled_opa.exists() {
                debug!("Using bundled OPA at: {:?}", bundled_opa);
                return Ok(bundled_opa);
            }
        }
    }

    // 3. Fall back to system PATH
    //
    // Note: We return the command name without pre-validation. This is intentional and follows
    // standard Rust practice (used by cargo, rustup, git2-rs, etc.) for several reasons:
    //
    // - Avoids TOCTOU issues: Pre-checking with `which` doesn't prevent execution failures
    //   (PATH can change, file can be deleted, permissions can change between check and use)
    // - Better error context: Execution failure (line 244) provides actual error with helpful
    //   message suggesting installation and --opa-path flag
    // - No false negatives: Pre-validation can't check OPA version compatibility, only that
    //   *some* executable exists in PATH
    //
    // If OPA is not found or fails to execute, the error is caught at line 244-246 with a
    // helpful message directing the user to install OPA or use --opa-path.
    debug!("Using OPA from system PATH");
    Ok(if cfg!(windows) {
        PathBuf::from("opa.exe")
    } else {
        PathBuf::from("opa")
    })
}

/// Compile all policies into a single unified WASM module using OPA
pub async fn compile_policies(
    policies: &[PolicyUnit],
    opa_path_override: Option<PathBuf>,
) -> Result<Vec<u8>> {
    compile_policies_with_namespace(policies, "cupcake.system", opa_path_override).await
}

/// Compile policies with a specific namespace for the entrypoint
pub async fn compile_policies_with_namespace(
    policies: &[PolicyUnit],
    namespace: &str,
    opa_path_override: Option<PathBuf>,
) -> Result<Vec<u8>> {
    if policies.is_empty() {
        bail!("No policies to compile");
    }

    info!(
        "Compiling {} policies into unified WASM module",
        policies.len()
    );

    // Create a temporary directory for the compilation that auto-cleans on drop
    let temp_dir = TempDir::new().context("Failed to create temp directory for OPA compilation")?;

    // DON'T canonicalize on Windows to avoid UNC paths (\\?\C:\...)
    // OPA has path handling bugs with both UNC paths and drive letters
    let temp_path = temp_dir.path();
    debug!("Using temp directory: {:?}", temp_path);

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

    // Copy helpers directory if it exists (required by refactored builtins)
    let helpers_src = policies_root.join("helpers");
    if helpers_src.exists() && helpers_src.is_dir() {
        debug!("Copying helpers directory: {:?}", helpers_src);
        let helpers_dest = temp_path.join("helpers");
        tokio::fs::create_dir_all(&helpers_dest).await?;

        // Copy all .rego files from helpers directory
        let mut helpers_dir = tokio::fs::read_dir(&helpers_src).await?;
        while let Some(entry) = helpers_dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("rego") {
                let file_name = path.file_name().unwrap();
                let dest_path = helpers_dest.join(file_name);
                tokio::fs::copy(&path, &dest_path).await?;
                debug!("Copied helper: {:?} -> {:?}", path, dest_path);
            }
        }
    }

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
    let opa_path = find_opa_binary(opa_path_override)?;
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
    let temp_path_str = temp_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to convert temp path to string"))?;

    let temp_path_arg = if cfg!(windows) {
        // On Windows, use file:// URL format to work around OPA bug #4174
        // OPA strips drive letters from normal Windows paths
        // Convert to file:// URL: C:\path -> file:///C:/path
        let url_path = temp_path_str.replace('\\', "/");
        format!("file:///{url_path}")
    } else {
        temp_path_str.to_string()
    };

    debug!("Temp path for OPA: {:?}", temp_path_arg);
    opa_cmd.arg(&temp_path_arg);

    // Output to bundle.tar.gz in temp dir
    let bundle_path = temp_path.join("bundle.tar.gz");
    let bundle_path_str = bundle_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to convert bundle path to string"))?;

    // On Windows, OPA can't write to file:// URLs, so use relative path
    // and set working directory to the temp directory
    let bundle_path_arg = if cfg!(windows) {
        // Use relative path "bundle.tar.gz" and set cwd to temp_path
        opa_cmd.current_dir(temp_path);
        "bundle.tar.gz".to_string()
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
            format!("stderr: {stderr}")
        } else if !stdout.is_empty() {
            format!("stdout: {stdout}")
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
    // Use a different temp file since the compilation temp_dir will be auto-deleted
    let debug_wasm_path =
        std::env::temp_dir().join(format!("cupcake-debug-{}.wasm", std::process::id()));
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

    // Create temp directory for extraction that auto-cleans on drop
    let temp_dir =
        TempDir::new().context("Failed to create temp directory for bundle extraction")?;
    let extract_path = temp_dir.path();

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

