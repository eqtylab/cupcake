# Plan for plan 005

Created: 2025-07-11

## Approach

This plan details the refactoring of Cupcake's policy format from a monolithic `cupcake.toml` file to a scalable, composable YAML-based system. The new system will be centered around a root `guardrails/cupcake.yaml` file that imports multiple policy fragments from a `guardrails/policies/` directory. This aligns with modern Infrastructure-as-Code (IaC) practices, promoting modularity and clear ownership of policy domains.

The implementation will involve replacing the `toml` crate with `serde_yaml_ng`, overhauling the policy loading and composition logic in `src/config/loader.rs`, and updating all related data structures, CLI commands, tests, and documentation. We will not maintain backward compatibility with the TOML format.

The core of the new loader will be a three-step process:

1.  **Discover:** Find the root `cupcake.yaml` and use its `imports` key with glob patterns to find all policy fragment files.
2.  **Compose:** Parse each YAML fragment and perform a deep merge, concatenating policy lists under their respective `HookEvent` and `Matcher` keys.
3.  **Validate:** Ensure all policy names are unique across the entire composed set to prevent conflicts.

The final output will be a single, in-memory policy structure that the evaluation engine can process, keeping the engine itself decoupled from the file format.

## Steps

### 1. Project Setup and Dependencies

1.  **Modify `Cargo.toml`:**
    - Remove the `toml` dependency.
    - Add `serde_yaml_ng = "0.10.0"` (or latest compatible version).
    - Add `glob = "0.3.2"` for resolving import patterns.
2.  **Update Error Handling (`src/error.rs`):**
    - Remove the `TomlSerialization` variant from `CupcakeError`.
    - Add a new `YamlSerialization(#[from] serde_yaml_ng::Error)` variant.

### 2. Redefine Configuration Data Structures (`src/config/types.rs`)

1.  **Create `RootConfig` struct:** This struct will represent `guardrails/cupcake.yaml`.
    ```rust
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RootConfig {
        #[serde(default)]
        pub settings: Settings,
        #[serde(default)]
        pub imports: Vec<String>,
    }
    ```
2.  **Simplify `Policy` struct:** The `hook_event` and `matcher` fields will be removed, as this context is now derived from the YAML structure.
    ```rust
    // The new Policy struct as it appears in YAML fragments
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Policy {
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        pub conditions: Vec<Condition>,
        pub action: Action,
    }
    ```
3.  **Define `PolicyFragment` type alias:** This will represent the "Grouped Map" structure of a single policy file.
    ```rust
    // Represents a single policies/*.yaml file
    pub type PolicyFragment = std::collections::HashMap<String, std::collections::HashMap<String, Vec<Policy>>>;
    ```
4.  **Create `ComposedPolicy` struct:** This struct will be the output of the loader and the input to the evaluation engine, re-introducing the context.
    ```rust
    // The final, flattened policy structure for the engine
    #[derive(Debug, Clone)]
    pub struct ComposedPolicy {
        pub name: String,
        pub description: Option<String>,
        pub hook_event: HookEventType,
        pub matcher: String,
        pub conditions: Vec<Condition>,
        pub action: Action,
    }
    ```
5.  **Remove `PolicyFile` struct:** This top-level TOML struct is now obsolete.

### 3. Overhaul Policy Loader (`src/config/loader.rs`)

1.  **Replace `load_policy_file` and `load_policy_hierarchy`** with a new primary function: `pub fn load_and_compose_policies(&mut self, start_dir: &Path) -> Result<Vec<ComposedPolicy>>`.
2.  **Implement Root Config Discovery:**
    - Search upwards from `start_dir` for a `guardrails/cupcake.yaml` file.
    - If not found, return an empty `Vec` or an error.
3.  **Implement Import Resolution:**
    - Parse the found `cupcake.yaml` into a `RootConfig` struct using `serde_yaml_ng`.
    - Get the directory of the root config file to resolve relative glob paths.
    - Use the `glob` crate to expand each pattern in `imports` into a list of file paths.
    - Sort the collected file paths alphabetically to ensure deterministic loading.
4.  **Implement Composition Logic:**
    - Initialize an empty `PolicyFragment` (a nested `HashMap`) to hold the composed result.
    - Iterate through the sorted policy fragment files:
      - Read and parse each file into a temporary `PolicyFragment` using `serde_yaml_ng`.
      - Perform a deep merge: iterate through the `(HookEvent, Matcher, Policies)` of the fragment and **append** the `Vec<Policy>` to the corresponding entry in the main composed map.
5.  **Implement Validation and Flattening:**
    - After merging all fragments, create a new function to validate and flatten the composed map.
    - Initialize an empty `HashSet<String>` to track policy names.
    - Iterate through the composed map. For each policy:
      - Check if its `name` is already in the `HashSet`. If so, return a `CupcakeError::Config` with a "duplicate policy name" error.
      - Add the name to the `HashSet`.
      - Create a `ComposedPolicy` instance, populating it with the `hook_event` and `matcher` from the map keys, and the details from the `Policy` struct.
      - Add the `ComposedPolicy` to a final `Vec`.
    - Return the `Vec<ComposedPolicy>`.

### 4. Update CLI Commands and Engine

1.  **`src/cli/app.rs`:**
    - Update default values for `policy_file` arguments from `cupcake.toml` to `guardrails/cupcake.yaml`.
    - Update help text to reflect the new format.
2.  **`src/cli/commands/run.rs`:**
    - Modify `load_policies` to call the new `PolicyLoader::load_and_compose_policies`.
    - The rest of the `execute` function will now receive a `Vec<ComposedPolicy>`.
3.  **`src/engine/evaluation.rs`:**
    - Update `build_ordered_policy_list` to work with `&Vec<ComposedPolicy>` instead of `&[PolicyFile]`. The logic will be simpler as it no longer needs to iterate through `PolicyFile`.
4.  **`src/cli/commands/validate.rs`:**
    - Update its `execute` method to use the new loader. It will now validate the entire composed policy set from `guardrails/`.
5.  **`src/cli/commands/init.rs`:**
    - Update its `execute` method to generate the new directory structure:
      - Create `guardrails/` directory.
      - Create `guardrails/cupcake.yaml` with default settings and an import for `policies/*.yaml`.
      - Create `guardrails/policies/` directory.
      - Create a placeholder `guardrails/policies/00-base.yaml`.

### 5. Update Tests and Documentation

1.  **Refactor `tests/serialization_tests.rs`:**
    - Remove all `toml::` tests.
    - Add new tests using `serde_yaml_ng` to verify serialization and deserialization of `RootConfig` and `PolicyFragment`.
2.  **Refactor `tests/run_command_integration_test.rs`:**
    - Modify tests to create a `guardrails/` directory with `cupcake.yaml` and a policy fragment file, instead of a single `test-policy.toml`.
3.  **Update `tests/claude-code-integration-directory/`:**
    - Delete `.claude/test-policy.toml`.
    - Create `guardrails/cupcake.yaml` with an `imports` key.
    - Create `guardrails/policies/01-test-policies.yaml`.
    - Convert all policies from the old TOML file into the new "Grouped Map" YAML format within `01-test-policies.yaml`.
4.  **Update `tests/integration_test_suite.sh`:**
    - Change all file paths and `cupcake validate` commands to point to the new YAML files.
5.  **Update Project Documentation:**
    - Modify `README.md`, `cupcake.md`, and `CLAUDE.md` to remove references to `cupcake.toml`.
    - Replace all TOML examples with the new, composable YAML format.
    - Clearly document the `guardrails/` directory convention and the structure of `cupcake.yaml` and policy fragments.

## Technical Decisions

- **YAML Library:** We will use `serde_yaml_ng` as it is a well-maintained fork of the original `serde_yaml` that binds to the native `libyaml` C library, offering robust parsing and performance.
- **Composition Strategy:** Policies will be loaded in alphabetical order of their filenames. Policy lists from different files that share the same `HookEvent` and `Matcher` will be **concatenated**. This is a simple, predictable, and powerful merge strategy that avoids complex merge conflicts.
- **Unique Policy Names:** Enforcing unique names across all composed policies is critical for preventing accidental overrides and ensuring a clear audit trail. The loader will be responsible for this validation.
- **Directory Convention:** We will standardize on the `guardrails/` directory name as it clearly communicates the purpose of the configuration. This is a convention, but the `cupcake` binary will search for it by default.
- **Decoupling Loader from Engine:** The loader will be responsible for translating the "Grouped Map" YAML structure into a flat list of `ComposedPolicy` structs. This ensures the evaluation engine remains simple and is not tightly coupled to the specifics of the file format.
