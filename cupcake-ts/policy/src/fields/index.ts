/**
 * Field types module - tool-specific and event-specific field definitions.
 */

import type { Tool } from '../core/index';
import type { BashFields } from './bash';
import type { WriteFields } from './write';
import type { EditFields } from './edit';
import type { ReadFields } from './read';
import type { TaskFields } from './task';

// Tool-specific fields
export { type CommonFields } from './common';
export { type BashFields } from './bash';
export { type WriteFields } from './write';
export { type EditFields } from './edit';
export { type ReadFields } from './read';
export { type TaskFields } from './task';
export { type ContextFields } from './context';

// Event-specific fields
export { type PostToolUseFields, type ToolResponseFields } from './postToolUse';
export { type UserPromptSubmitFields } from './userPrompt';
export { type StopFields } from './stop';
export { type SessionStartFields } from './sessionStart';

/**
 * Maps tool names to their field types.
 */
export interface ToolFieldsMap {
  Bash: BashFields;
  Write: WriteFields;
  Edit: EditFields;
  Read: ReadFields;
  Task: TaskFields;
}

/**
 * Get the fields type for a tool or array of tools.
 * When multiple tools are specified, returns the union of their fields.
 *
 * @example
 * ```typescript
 * type F1 = FieldsFor<'Bash'>;           // BashFields
 * type F2 = FieldsFor<['Write', 'Edit']>; // WriteFields | EditFields
 * ```
 */
export type FieldsFor<T extends Tool | readonly Tool[]> =
  T extends readonly Tool[]
    ? ToolFieldsMap[T[number]]
    : T extends Tool
      ? ToolFieldsMap[T]
      : never;
