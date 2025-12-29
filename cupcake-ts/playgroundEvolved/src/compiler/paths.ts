/**
 * Field path → Rego path mapping.
 */

import { toSnakeCase } from './format';

/**
 * Maps DSL field names to Rego input paths.
 */
const PATH_MAP: Record<string, string> = {
  // Tool input fields
  command: 'input.tool_input.command',
  path: 'input.tool_input.file_path', // Claude Code tools use file_path
  filePath: 'input.tool_input.file_path', // Alias for clarity
  content: 'input.tool_input.content',
  oldString: 'input.tool_input.old_string',
  newString: 'input.tool_input.new_string',
  description: 'input.tool_input.description',
  prompt: 'input.tool_input.prompt', // Task tool's prompt

  // Preprocessing fields
  isSymlink: 'input.is_symlink',
  resolvedFilePath: 'input.resolved_file_path',
  originalFilePath: 'input.original_file_path',

  // Context fields
  hookEventName: 'input.hook_event_name',
  userPrompt: 'input.user_prompt',
  sessionId: 'input.session_id',
  cwd: 'input.cwd',

  // PostToolUse fields (tool_response)
  'toolResponse.stdout': 'input.tool_response.stdout',
  'toolResponse.stderr': 'input.tool_response.stderr',
  'toolResponse.exitCode': 'input.tool_response.exit_code',

  // UserPromptSubmit fields
  submittedPrompt: 'input.prompt', // The user's submitted prompt

  // Stop/SubagentStop fields
  stopHookActive: 'input.stop_hook_active',
};

/**
 * Result of compiling a field path.
 */
export interface CompiledPath {
  /** The base Rego path without transforms */
  readonly basePath: string;
  /** The full Rego path with transforms applied */
  readonly fullPath: string;
  /** Any transforms applied (e.g., ['lower']) */
  readonly transforms: readonly string[];
}

/**
 * Compiles a field path to its Rego representation.
 *
 * @param path - Path array, e.g., ['input', 'resolvedFilePath', '__lower']
 *               or ['input', 'toolResponse', 'stdout']
 * @returns Compiled path information
 */
export function compileFieldPath(path: readonly string[]): CompiledPath {
  // Handle signals: ["input", "signals", "gitBranch"]
  if (path.length >= 3 && path[1] === 'signals') {
    const basePath = path.slice(0, 3).join('.');
    const transforms = path.slice(3).filter((p) => p.startsWith('__'));
    return {
      basePath,
      fullPath: applyTransforms(basePath, transforms),
      transforms,
    };
  }

  // Separate field segments from transforms
  const fieldSegments: string[] = [];
  const transforms: string[] = [];

  for (let i = 1; i < path.length; i++) {
    const segment = path[i];
    if (segment?.startsWith('__')) {
      transforms.push(segment);
    } else if (segment) {
      fieldSegments.push(segment);
    }
  }

  if (fieldSegments.length === 0) {
    return { basePath: 'input', fullPath: 'input', transforms: [] };
  }

  // Try to find a direct mapping for the full field path (e.g., "toolResponse.stdout")
  const fullFieldPath = fieldSegments.join('.');
  let basePath = PATH_MAP[fullFieldPath];

  if (!basePath) {
    // Try single field lookup, then append remaining segments
    const firstField = fieldSegments[0];
    const mappedFirst = PATH_MAP[firstField!] ?? `input.${toSnakeCase(firstField!)}`;

    if (fieldSegments.length === 1) {
      basePath = mappedFirst;
    } else {
      // Append remaining segments in snake_case
      const remaining = fieldSegments.slice(1).map(toSnakeCase).join('.');
      basePath = `${mappedFirst}.${remaining}`;
    }
  }

  return {
    basePath,
    fullPath: applyTransforms(basePath, transforms),
    transforms,
  };
}

/**
 * Applies transforms to a Rego path.
 */
function applyTransforms(path: string, transforms: readonly string[]): string {
  let result = path;
  for (const transform of transforms) {
    if (transform === '__lower') {
      result = `lower(${result})`;
    } else if (transform === '__upper') {
      result = `upper(${result})`;
    }
  }
  return result;
}

/**
 * Checks if a path has transforms applied.
 */
export function hasTransforms(path: readonly string[]): boolean {
  return path.some((p) => p.startsWith('__'));
}

/**
 * Derives a sensible local variable name from a field name.
 * Preserves semantic prefixes (like "resolved", "original") while shortening.
 *
 * Examples:
 * - resolvedFilePath -> resolved_path
 * - originalFilePath -> original_path
 * - filePath -> path
 * - command -> command
 */
export function deriveLocalVarName(fieldName: string): string {
  const snakeCase = toSnakeCase(fieldName);

  // Common shortenings - preserve semantic prefix, drop redundant "file"
  if (snakeCase.includes('_file_path')) {
    return snakeCase.replace('_file_path', '_path');
  }
  if (snakeCase === 'file_path') {
    return 'path';
  }

  return snakeCase;
}

/**
 * Derives the local variable name from a field path.
 * Finds the last non-transform segment and applies shortening.
 */
export function getLocalVarNameFromPath(path: readonly string[]): string {
  // Find the last non-transform segment
  for (let i = path.length - 1; i >= 0; i--) {
    const segment = path[i];
    if (segment && !segment.startsWith('__')) {
      return deriveLocalVarName(segment);
    }
  }
  return 'value';
}
