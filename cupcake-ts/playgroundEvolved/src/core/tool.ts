/**
 * Supported tool types that can be targeted by policy rules.
 * These correspond to Claude Code tool names.
 */
export type Tool = 'Write' | 'Edit' | 'Bash' | 'Read' | 'Task';

/**
 * All supported tools as an array (useful for iteration).
 */
export const ALL_TOOLS: readonly Tool[] = ['Write', 'Edit', 'Bash', 'Read', 'Task'] as const;
