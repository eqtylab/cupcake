2.  **Prioritize Implementing the `sync` Command:**
    - **Observation:** The `sync` command (`src/cli/commands/sync.rs`) is a crucial part of the user workflow but is currently a placeholder. Without it, users must manually edit their `.claude/settings.local.json`, which is error-prone and undermines the seamless experience Cupcake aims for.
    - **Recommendation:** The implementation should be robust:
      1.  Discover the correct `settings.json` or `settings.local.json` file.
      2.  Parse the JSON, carefully preserving all existing user settings (models, other hooks, etc.).
      3.  Intelligently insert or update the Cupcake hook configuration. It should be a single, generic hook for all tools (`"matcher": ""`) that calls `cupcake run`. This simplifies the hook config and puts all the matching logic inside Cupcake's YAML, which is the goal.
      4.  Write the modified JSON back, maintaining formatting as much as possible. This command is the key to user adoption.
