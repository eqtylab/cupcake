1.  **Refine Graceful Degradation in `run` Command:**
    - **Observation:** In `src/cli/commands/run.rs`, if reading stdin or loading the configuration fails, the process exits with code 0. This is a "fail-open" strategy, which prevents Cupcake from blocking a developer if it's misconfigured.
    - **Recommendation:** This is a reasonable default, but it fails silently. The user might not realize their guardrails are inactive. I recommend printing a large, highly visible warning to `stderr` in these error cases. For example:
      ```text
      🔥🔥🔥 CUPCAKE WARNING 🔥🔥🔥
      Error loading configuration: [error details].
      Guardrails are INACTIVE for this operation. Proceeding without enforcement.
      ```
    - This makes the failure explicit without being disruptive. You could even make this behavior configurable in `cupcake.yaml` (`on_error: fail_open | fail_closed`).
