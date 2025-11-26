/**
 * Vercel AI SDK integration with Cupcake policy enforcement
 *
 * This example shows how to use Cupcake to govern AI agent tool calls
 * in applications built with the Vercel AI SDK.
 *
 * The pattern demonstrated here applies to any tool-using agent framework.
 */

import { Cupcake } from '../index';
import { z } from 'zod';

// Initialize Cupcake
const cupcake = new Cupcake();

// Tool definition schemas
const shellToolSchema = z.object({
  command: z.string().describe('The shell command to execute'),
  args: z.array(z.string()).optional().describe('Command arguments'),
  cwd: z.string().optional().describe('Working directory'),
});

const databaseToolSchema = z.object({
  query: z.string().describe('SQL query to execute'),
  parameters: z.array(z.any()).optional().describe('Query parameters'),
});

const fileToolSchema = z.object({
  operation: z.enum(['read', 'write', 'delete']).describe('File operation'),
  path: z.string().describe('File path'),
  content: z.string().optional().describe('Content to write (for write operation)'),
});

/**
 * Wrapper that adds Cupcake policy enforcement to any tool
 */
function policyEnforcedTool<TInput, TOutput>(
  name: string,
  description: string,
  schema: z.ZodType<TInput>,
  execute: (input: TInput) => Promise<TOutput>,
) {
  return {
    name,
    description,
    parameters: schema,
    execute: async (input: TInput): Promise<TOutput> => {
      // Evaluate the tool call against Cupcake policies
      const decision = await cupcake.evaluate({
        kind: 'tool_use',
        tool: name,
        input: input as any,
        timestamp: new Date().toISOString(),
      });

      // Enforce policy decision
      if (decision.decision === 'Halt' || decision.decision === 'Deny') {
        throw new Error(`Policy blocked ${name}: ${decision.reason}`);
      }

      if (decision.decision === 'Ask') {
        throw new Error(`${name} requires approval: ${decision.question || decision.reason}`);
      }

      // Policy allows - execute the tool
      console.log(`✓ Policy allowed ${name}`);
      if (decision.context && decision.context.length > 0) {
        console.log('  Guidance:', decision.context.join('; '));
      }

      return execute(input);
    },
  };
}

/**
 * Example tools with policy enforcement
 */

const shellTool = policyEnforcedTool(
  'shell_execute',
  'Execute a shell command',
  shellToolSchema,
  async (input) => {
    // Actual implementation would execute the command
    console.log(`Executing: ${input.command} ${input.args?.join(' ') || ''}`);

    // For demonstration, just return simulated output
    return {
      stdout: `Command executed: ${input.command}`,
      stderr: '',
      exitCode: 0,
    };
  },
);

const databaseTool = policyEnforcedTool(
  'database_query',
  'Execute a database query',
  databaseToolSchema,
  async (input) => {
    console.log(`Executing query: ${input.query}`);

    // Actual implementation would query the database
    return {
      rows: [],
      rowCount: 0,
      query: input.query,
    };
  },
);

const fileTool = policyEnforcedTool(
  'file_operation',
  'Perform file operations (read, write, delete)',
  fileToolSchema,
  async (input) => {
    console.log(`File ${input.operation}: ${input.path}`);

    // Actual implementation would perform file operations
    switch (input.operation) {
      case 'read':
        return { content: 'file contents here' };
      case 'write':
        return { success: true, bytesWritten: input.content?.length || 0 };
      case 'delete':
        return { success: true };
      default:
        throw new Error(`Unknown operation: ${input.operation}`);
    }
  },
);

/**
 * Example agent configuration with Vercel AI SDK
 */
async function createPolicyEnforcedAgent() {
  // Initialize Cupcake
  await cupcake.init('.cupcake');
  console.log(`🧁 Cupcake ${cupcake.version} initialized\n`);

  // Return tools that are wrapped with policy enforcement
  return {
    tools: {
      shell_execute: shellTool,
      database_query: databaseTool,
      file_operation: fileTool,
    },
  };
}

/**
 * Example usage
 */
async function main() {
  const agent = await createPolicyEnforcedAgent();

  console.log('Agent tools:', Object.keys(agent.tools).join(', '));
  console.log();

  // Simulate tool calls from the AI agent
  console.log('=== Example 1: Safe shell command ===');
  try {
    const result = await agent.tools.shell_execute.execute({
      command: 'ls',
      args: ['-la'],
      cwd: '/tmp',
    });
    console.log('Result:', result);
  } catch (error: any) {
    console.error('Error:', error.message);
  }
  console.log();

  console.log('=== Example 2: Dangerous command (should be blocked) ===');
  try {
    const result = await agent.tools.shell_execute.execute({
      command: 'rm',
      args: ['-rf', '/'],
    });
    console.log('Result:', result);
  } catch (error: any) {
    console.error('Error:', error.message);
  }
  console.log();

  console.log('=== Example 3: Database query ===');
  try {
    const result = await agent.tools.database_query.execute({
      query: 'SELECT * FROM users WHERE id = $1',
      parameters: [123],
    });
    console.log('Result:', result);
  } catch (error: any) {
    console.error('Error:', error.message);
  }
  console.log();

  console.log('=== Example 4: File operation ===');
  try {
    const result = await agent.tools.file_operation.execute({
      operation: 'read',
      path: '/etc/passwd',
    });
    console.log('Result:', result);
  } catch (error: any) {
    console.error('Error:', error.message);
  }
  console.log();

  console.log('✅ Examples complete!');
}

// Run if executed directly
if (require.main === module) {
  main().catch((error) => {
    console.error('Fatal error:', error);
    process.exit(1);
  });
}

export { createPolicyEnforcedAgent, policyEnforcedTool };
