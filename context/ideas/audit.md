1.  **A First-Class Audit Trail:** The `audit` command is currently a placeholder, but the `engine/audit.rs` and `command_executor` modules lay the groundwork for a killer feature. A rich, queryable audit log is essential for enterprise adoption, security reviews, and debugging agent behavior.
    - **Vision:** The `cupcake audit` command should become a powerful query tool. Imagine commands like:
      - `cupcake audit --session <id>`: Show all policy decisions for a specific session.
      - `cupcake audit --policy "Block dangerous commands"`: Show every time a specific policy was triggered.
      - `cupcake audit --decision block`: Show all blocked actions across all sessions.
      - `cupcake audit --since 24h`: Review all agent activity from the last day.
    - This turns Cupcake from just an enforcer into an observability platform for AI agents.
