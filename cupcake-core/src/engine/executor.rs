//! Executor - Handles OS/IO operations for the engine.
//!
//! This module separates "Doing" (shell commands, process execution, filesystem I/O)
//! from "Thinking" (routing, WASM evaluation, synthesis) in the engine.
//!
//! The Executor is an ephemeral struct created for each evaluation, holding references
//! to the resources needed for signal gathering and action execution.

use anyhow::Result;
use futures::future::join_all;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, error, info, trace, warn};

use super::config::SHELL_COMMAND;
use super::decision::{DecisionObject, DecisionSet, FinalDecision};
use super::global_config;
use super::metadata::PolicyUnit;
use super::rulebook::{self, Rulebook};
use crate::debug::SignalTelemetry;
use crate::telemetry::span::SignalExecution;
use crate::trust::TrustVerifier;
use crate::watchdog::Watchdog;

/// Executor handles all OS/IO interactions for policy evaluation.
///
/// This is an ephemeral struct created for each evaluation. It holds references
/// to the resources needed for executing signals and actions, avoiding repeated
/// parameter passing through function signatures.
pub struct Executor<'a> {
    /// Rulebook containing signal and action definitions
    pub rulebook: Option<&'a Rulebook>,
    /// Global rulebook for global actions
    pub global_rulebook: Option<&'a Rulebook>,
    /// Trust verifier for script integrity checks
    pub trust_verifier: Option<&'a TrustVerifier>,
    /// Watchdog for LLM-as-Judge evaluation
    pub watchdog: Option<&'a Watchdog>,
    /// Working directory for command execution
    pub working_dir: &'a Path,
}

impl<'a> Executor<'a> {
    /// Gather signal values by executing shell commands.
    ///
    /// This function:
    /// 1. Collects required signals from matched policies
    /// 2. Auto-adds signals for builtin policies
    /// 3. Injects builtin configuration
    /// 4. Executes signals with trust verification
    /// 5. Runs watchdog evaluation if enabled
    #[tracing::instrument(
        skip(self, input, matched_policies, signal_telemetry),
        fields(
            signal_count = tracing::field::Empty,
            signals_executed = tracing::field::Empty,
            duration_ms = tracing::field::Empty
        )
    )]
    pub async fn gather_signals(
        &self,
        input: &Value,
        matched_policies: &[PolicyUnit],
        mut signal_telemetry: Option<&mut SignalTelemetry>,
    ) -> Result<Value> {
        let start = Instant::now();

        // Collect all unique required signals from matched policies
        let mut required_signals = std::collections::HashSet::new();
        for policy in matched_policies {
            for signal_name in &policy.routing.required_signals {
                required_signals.insert(signal_name.clone());
            }
        }

        // Auto-add signals for builtin policies
        if let Some(rulebook) = self.rulebook {
            for policy in matched_policies {
                if policy
                    .package_name
                    .starts_with("cupcake.policies.builtins.")
                {
                    let builtin_name = policy
                        .package_name
                        .strip_prefix("cupcake.policies.builtins.")
                        .unwrap_or("");

                    // Special handling for post_edit_check
                    if builtin_name == "post_edit_check" {
                        if let Some(signal_name) = rulebook.builtins.get_post_edit_signal(input) {
                            if rulebook.signals.contains_key(&signal_name) {
                                debug!(
                                    "Auto-adding signal '{}' for post_edit_check builtin",
                                    signal_name
                                );
                                required_signals.insert(signal_name);
                            }
                        }
                    } else {
                        // For other builtins, add all matching signals
                        let signal_prefix = format!("__builtin_{builtin_name}_");
                        for signal_name in rulebook.signals.keys() {
                            if signal_name.starts_with(&signal_prefix) {
                                debug!(
                                    "Auto-adding signal '{}' for builtin '{}'",
                                    signal_name, builtin_name
                                );
                                required_signals.insert(signal_name.clone());
                            }
                        }
                    }
                }
            }
        }

        // Always inject builtin config, even when no signals are required
        let mut enriched_input = input.clone();
        if let Some(input_obj) = enriched_input.as_object_mut() {
            let mut builtin_config = serde_json::Map::new();

            // First inject project configs (baseline)
            if let Some(project_rulebook) = self.rulebook {
                debug!("Injecting builtin configs from project rulebook");
                builtin_config.extend(project_rulebook.builtins.to_json_configs());
            }

            // Then inject global configs (override project)
            if let Some(global_rulebook) = self.global_rulebook {
                debug!("Injecting builtin configs from global rulebook (overrides project)");
                builtin_config.extend(global_rulebook.builtins.to_json_configs());
            }

            if !builtin_config.is_empty() {
                debug!(
                    "Injected {} builtin configurations total",
                    builtin_config.len()
                );
                input_obj.insert(
                    "builtin_config".to_string(),
                    serde_json::Value::Object(builtin_config),
                );
            }
        }

        // Check if watchdog should run
        let is_pre_action_event = input
            .get("hook_event_name")
            .and_then(|v| v.as_str())
            .map(|s| {
                matches!(
                    s,
                    "PreToolUse" | "beforeShellExecution" | "beforeMCPExecution"
                )
            })
            .unwrap_or(false);
        let watchdog_should_run = self.watchdog.is_some() && is_pre_action_event;

        if required_signals.is_empty() && !watchdog_should_run {
            debug!("No signals required and watchdog not enabled - returning with builtin config");
            return Ok(enriched_input);
        }

        let signal_names: Vec<String> = required_signals.into_iter().collect();
        info!(
            "Gathering {} signals: {:?}",
            signal_names.len(),
            signal_names
        );

        // Execute signals if we have a rulebook
        let signal_data = if let Some(rulebook) = self.rulebook {
            self.execute_signals_with_trust(
                &signal_names,
                rulebook,
                input,
                signal_telemetry.as_deref_mut(),
            )
            .await
            .unwrap_or_else(|e| {
                warn!("Signal execution failed: {}", e);
                HashMap::new()
            })
        } else {
            debug!("No rulebook available - no signals collected");
            HashMap::new()
        };

        // Merge signal data into enriched input
        let mut signal_count = signal_data.len();
        if let Some(input_obj) = enriched_input.as_object_mut() {
            let mut signals_obj = serde_json::to_value(signal_data)?;

            // Execute Watchdog if enabled
            if let Some(watchdog) = self.watchdog {
                if is_pre_action_event {
                    debug!(
                        "Executing Watchdog evaluation for {:?} event",
                        input.get("hook_event_name")
                    );
                    let watchdog_start = Instant::now();
                    let watchdog_input = Watchdog::input_from_event(input);
                    let watchdog_output = watchdog.evaluate(watchdog_input).await;
                    let watchdog_duration = watchdog_start.elapsed();
                    debug!(
                        "Watchdog result: allow={}, confidence={}",
                        watchdog_output.allow, watchdog_output.confidence
                    );

                    // Capture watchdog in telemetry
                    if let Some(ref mut telemetry) = signal_telemetry {
                        telemetry.signals.push(SignalExecution {
                            name: "watchdog".to_string(),
                            command: format!("LLM evaluation via {}", watchdog.backend_name()),
                            result: serde_json::to_value(&watchdog_output).unwrap_or_default(),
                            duration_ms: Some(watchdog_duration.as_millis() as u64),
                            exit_code: None,
                        });
                    }

                    if let Some(signals_map) = signals_obj.as_object_mut() {
                        signals_map.insert(
                            "watchdog".to_string(),
                            serde_json::to_value(&watchdog_output)?,
                        );
                        signal_count += 1;
                    }
                }
            }

            input_obj.insert("signals".to_string(), signals_obj);
        }

        // Record span fields
        let duration = start.elapsed();
        let current_span = tracing::Span::current();
        current_span.record("signal_count", signal_count);
        current_span.record("signals_executed", signal_names.join(",").as_str());
        current_span.record("duration_ms", duration.as_millis());

        debug!("Input enriched with {} signal values", signal_count);
        trace!(
            signal_count = signal_count,
            duration_ms = duration.as_millis(),
            "Signal gathering complete"
        );

        Ok(enriched_input)
    }

    /// Execute signals with trust verification.
    async fn execute_signals_with_trust(
        &self,
        signal_names: &[String],
        rulebook: &Rulebook,
        event_data: &Value,
        mut signal_telemetry: Option<&mut SignalTelemetry>,
    ) -> Result<HashMap<String, Value>> {
        if signal_names.is_empty() {
            return Ok(HashMap::new());
        }

        debug!(
            "Executing {} signals with trust verification",
            signal_names.len()
        );

        let futures: Vec<_> = signal_names
            .iter()
            .map(|name| {
                let name = name.clone();
                let trust_verifier = self.trust_verifier.cloned();
                let signal_config = rulebook.get_signal(&name).cloned();
                let event_data = event_data.clone();

                async move {
                    let signal = match signal_config {
                        Some(s) => s,
                        None => {
                            return (
                                name.clone(),
                                Err(anyhow::anyhow!("Signal '{}' not found", name)),
                                None,
                            );
                        }
                    };

                    // Verify trust if enabled
                    if let Some(verifier) = &trust_verifier {
                        if let Err(e) = verifier.verify_script(&signal.command).await {
                            return (
                                name.clone(),
                                Err(anyhow::anyhow!("Trust verification failed: {}", e)),
                                None,
                            );
                        }
                    }

                    // Execute the signal and measure time
                    let signal_start = Instant::now();
                    let result = rulebook.execute_signal_with_input(&name, &event_data).await;
                    let signal_duration = signal_start.elapsed();

                    let signal_execution = SignalExecution {
                        name: name.clone(),
                        command: signal.command.clone(),
                        result: result.as_ref().unwrap_or(&Value::Null).clone(),
                        duration_ms: Some(signal_duration.as_millis() as u64),
                        exit_code: None,
                    };

                    (name, result, Some(signal_execution))
                }
            })
            .collect();

        let results = join_all(futures).await;

        let mut signal_data = HashMap::new();
        for (name, result, signal_execution) in results {
            match result {
                Ok(value) => {
                    debug!("Signal '{}' executed successfully", name);
                    signal_data.insert(name, value);

                    if let (Some(ref mut telemetry), Some(execution)) =
                        (&mut signal_telemetry, signal_execution)
                    {
                        telemetry.signals.push(execution);
                    }
                }
                Err(e) => {
                    error!("Signal '{}' failed: {}", name, e);

                    if let (Some(ref mut telemetry), Some(execution)) =
                        (&mut signal_telemetry, signal_execution)
                    {
                        telemetry.signals.push(execution);
                    }
                }
            }
        }

        Ok(signal_data)
    }

    /// Execute actions based on the final decision (uses project rulebook).
    pub async fn execute_actions(&self, final_decision: &FinalDecision, decision_set: &DecisionSet) {
        let Some(rulebook) = self.rulebook else {
            debug!("No rulebook available - no actions to execute");
            return;
        };

        self.execute_actions_with_rulebook(final_decision, decision_set, rulebook)
            .await;
    }

    /// Execute global actions based on the final decision (uses global rulebook).
    pub async fn execute_global_actions(
        &self,
        final_decision: &FinalDecision,
        decision_set: &DecisionSet,
    ) {
        let Some(rulebook) = self.global_rulebook else {
            debug!("No global rulebook available - no global actions to execute");
            return;
        };

        self.execute_actions_with_rulebook(final_decision, decision_set, rulebook)
            .await;
    }

    /// Gather signals for global policy evaluation.
    ///
    /// This uses the global rulebook for signal definitions and builtin configs.
    pub async fn gather_global_signals(
        &self,
        input: &Value,
        matched_policies: &[PolicyUnit],
    ) -> Result<Value> {
        let Some(rulebook) = self.global_rulebook else {
            debug!("No global rulebook - returning input unchanged");
            return Ok(input.clone());
        };

        // Collect required signals from matched policies
        let mut required_signals = std::collections::HashSet::new();
        for policy in matched_policies {
            for signal_name in &policy.routing.required_signals {
                required_signals.insert(signal_name.clone());
            }
        }

        // Auto-add signals for global builtin policies
        for policy in matched_policies {
            let is_global_builtin = policy
                .package_name
                .starts_with("cupcake.global.policies.builtins.");
            let is_project_builtin = policy
                .package_name
                .starts_with("cupcake.policies.builtins.");

            if is_global_builtin || is_project_builtin {
                let prefix = if is_global_builtin {
                    "cupcake.global.policies.builtins."
                } else {
                    "cupcake.policies.builtins."
                };

                let builtin_name = policy.package_name.strip_prefix(prefix).unwrap_or("");

                // Add all signals that match this builtin's pattern
                let signal_prefix = format!("__builtin_{builtin_name}_");
                for signal_name in rulebook.signals.keys() {
                    if signal_name.starts_with(&signal_prefix) {
                        debug!(
                            "Auto-adding signal '{}' for global builtin '{}'",
                            signal_name, builtin_name
                        );
                        required_signals.insert(signal_name.clone());
                    }
                }

                // Also check for signals without the trailing underscore
                let signal_prefix_no_underscore = format!("__builtin_{builtin_name}");
                for signal_name in rulebook.signals.keys() {
                    if signal_name.starts_with(&signal_prefix_no_underscore)
                        && !signal_name.starts_with(&signal_prefix)
                    {
                        debug!(
                            "Auto-adding signal '{}' for global builtin '{}'",
                            signal_name, builtin_name
                        );
                        required_signals.insert(signal_name.clone());
                    }
                }
            }
        }

        // Always inject builtin config from global rulebook
        let mut enriched_input = input.clone();
        if let Some(input_obj) = enriched_input.as_object_mut() {
            debug!("Injecting builtin configs from global rulebook");
            let builtin_config = rulebook.builtins.to_json_configs();

            if !builtin_config.is_empty() {
                debug!("Injected {} builtin configurations", builtin_config.len());
                input_obj.insert(
                    "builtin_config".to_string(),
                    serde_json::Value::Object(builtin_config),
                );
            }
        }

        if required_signals.is_empty() {
            debug!("No global signals required - returning with builtin config");
            return Ok(enriched_input);
        }

        let signal_names: Vec<String> = required_signals.into_iter().collect();
        info!("Gathering {} global signals", signal_names.len());

        // Execute signals using global rulebook
        let signal_data = self
            .execute_signals_with_trust(&signal_names, rulebook, input, None)
            .await
            .unwrap_or_else(|e| {
                warn!("Global signal execution failed: {}", e);
                HashMap::new()
            });

        // Merge signal data into enriched input
        if let Some(obj) = enriched_input.as_object_mut() {
            obj.insert("signals".to_string(), serde_json::json!(signal_data));
        }

        Ok(enriched_input)
    }

    /// Execute actions with a specific rulebook.
    async fn execute_actions_with_rulebook(
        &self,
        final_decision: &FinalDecision,
        decision_set: &DecisionSet,
        rulebook: &Rulebook,
    ) {
        info!(
            "execute_actions_with_rulebook called with decision: {:?}",
            final_decision
        );

        // Determine working directory
        let is_global = self
            .global_rulebook
            .map(|gb| std::ptr::eq(gb as *const _, rulebook as *const _))
            .unwrap_or(false);

        let working_dir = if is_global {
            if let Ok(Some(global_paths)) = global_config::GlobalPaths::discover() {
                global_paths.root
            } else {
                self.working_dir.to_path_buf()
            }
        } else {
            self.working_dir.to_path_buf()
        };

        match final_decision {
            FinalDecision::Halt { reason, .. } => {
                info!("Executing actions for HALT decision: {}", reason);
                self.execute_rule_specific_actions(&decision_set.halts, rulebook, &working_dir)
                    .await;
            }
            FinalDecision::Deny { reason, .. } => {
                info!("Executing actions for DENY decision: {}", reason);

                for action in &rulebook.actions.on_any_denial {
                    self.execute_single_action(action, &working_dir).await;
                }

                self.execute_rule_specific_actions(&decision_set.denials, rulebook, &working_dir)
                    .await;
            }
            FinalDecision::Block { reason, .. } => {
                info!("Executing actions for BLOCK decision: {}", reason);
                self.execute_rule_specific_actions(&decision_set.blocks, rulebook, &working_dir)
                    .await;
            }
            FinalDecision::Ask { .. } => {
                debug!("ASK decision - no automatic actions");
            }
            FinalDecision::Allow { .. } => {
                debug!("ALLOW decision - no actions needed");
            }
            FinalDecision::Modify { .. } => {
                debug!("MODIFY decision - no actions needed");
            }
        }
    }

    /// Execute actions for a specific set of decision objects.
    async fn execute_rule_specific_actions(
        &self,
        decisions: &[DecisionObject],
        rulebook: &Rulebook,
        working_dir: &std::path::PathBuf,
    ) {
        info!(
            "execute_rule_specific_actions: Checking actions for {} decision objects",
            decisions.len()
        );

        for decision_obj in decisions {
            let rule_id = &decision_obj.rule_id;

            if let Some(actions) = rulebook.actions.by_rule_id.get(rule_id) {
                info!("Found {} actions for rule {}", actions.len(), rule_id);
                for action in actions {
                    self.execute_single_action(action, working_dir).await;
                }
            }
        }
    }

    /// Execute a single action command.
    async fn execute_single_action(
        &self,
        action: &rulebook::ActionConfig,
        working_dir: &std::path::PathBuf,
    ) {
        debug!(
            "Executing action: {} in directory: {:?}",
            action.command, working_dir
        );

        // Verify trust if enabled
        if let Some(verifier) = self.trust_verifier {
            if let Err(e) = verifier.verify_script(&action.command).await {
                error!("Action blocked by trust verification: {}", e);
                return;
            }
        }

        let working_dir = working_dir.clone();
        let command = action.command.clone();

        tokio::spawn(async move {
            let is_script_path = (command.starts_with('/') || command.starts_with("./"))
                && !command.contains("&&")
                && !command.contains("||")
                && !command.contains(';')
                && !command.contains('|')
                && !command.contains('>');

            let is_shell_script = command.ends_with(".sh");

            let result = if is_script_path && !is_shell_script {
                let script_path = std::path::Path::new(&command);
                let script_working_dir = script_path
                    .parent()
                    .and_then(|p| p.parent())
                    .and_then(|p| p.parent())
                    .unwrap_or(&working_dir);

                tokio::process::Command::new(&command)
                    .current_dir(script_working_dir)
                    .output()
                    .await
            } else if is_shell_script && cfg!(windows) {
                let script_path = std::path::Path::new(&command);
                let script_working_dir = script_path
                    .parent()
                    .and_then(|p| p.parent())
                    .and_then(|p| p.parent())
                    .unwrap_or(&working_dir);

                let bash_path = if command.len() >= 3 && command.chars().nth(1) == Some(':') {
                    let drive = command.chars().next().unwrap().to_lowercase();
                    let path_part = &command[2..].replace('\\', "/");
                    format!("/{drive}{path_part}")
                } else {
                    command.replace('\\', "/")
                };

                tokio::process::Command::new(*SHELL_COMMAND)
                    .arg(&bash_path)
                    .current_dir(script_working_dir)
                    .output()
                    .await
            } else {
                tokio::process::Command::new(*SHELL_COMMAND)
                    .arg("-c")
                    .arg(&command)
                    .current_dir(&working_dir)
                    .output()
                    .await
            };

            match result {
                Ok(output) => {
                    if output.status.success() {
                        debug!("Action completed successfully: {}", command);
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        warn!(
                            "Action failed: {} - stderr: {} stdout: {}",
                            command, stderr, stdout
                        );
                    }
                }
                Err(e) => {
                    error!("Failed to execute action: {} - {}", command, e);
                }
            }
        });

        tokio::task::yield_now().await;
    }
}
