Formalize the Policy Language with a Schema: The YAML format is effective but implicitly defined by the serde structs.
Recommendation: Publish a formal JSON Schema for the policy language. This would enable IDE validation, autocompletion in tools like VS Code, and programmatic generation of policies, further improving the user experience and reducing configuration errors.
Expand Sandboxing Capabilities: The stubs for advanced sandboxing are in place.
Recommendation: Prioritize the implementation of seccomp-bpf filters for shell mode on Linux. This would be a game-changing security feature, allowing administrators to define exactly which system calls a shell script is allowed to make, effectively neutering entire classes of attack even if shell injection were to occur.
Enhance Observability: The debug log is excellent for troubleshooting but not for monitoring.
Recommendation: Introduce structured logging (e.g., JSON format) as an option. Add metrics for key events: policy evaluation latency, number of policies triggered (by name), and hard vs. soft blocks. This would provide invaluable operational intelligence in a production environment.
