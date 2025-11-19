/**
 * Basic Cupcake usage example
 *
 * This example demonstrates synchronous evaluation in a CLI script.
 * For web servers or async contexts, see async-server.ts
 */

import { Cupcake } from '../index';

async function main() {
  console.log('🧁 Cupcake Basic Example\n');

  // Initialize Cupcake (async to allow OPA download if needed)
  const cupcake = new Cupcake();
  console.log('Initializing Cupcake...');
  await cupcake.init('.cupcake', 'claude');
  console.log(`✓ Cupcake ${cupcake.version} ready\n`);

  // Example 1: Evaluate a safe command
  console.log('Example 1: Safe command');
  const safeEvent = {
    kind: 'shell',
    command: 'ls',
    args: ['-la'],
    cwd: '/tmp',
  };

  const safeDecision = await cupcake.evaluate(safeEvent);
  console.log('Event:', JSON.stringify(safeEvent, null, 2));
  console.log('Decision:', safeDecision.decision);
  if (safeDecision.reason) {
    console.log('Reason:', safeDecision.reason);
  }
  console.log();

  // Example 2: Evaluate a potentially dangerous command
  console.log('Example 2: Dangerous command');
  const dangerousEvent = {
    kind: 'shell',
    command: 'rm',
    args: ['-rf', '/'],
    cwd: '/tmp',
  };

  const dangerousDecision = await cupcake.evaluate(dangerousEvent);
  console.log('Event:', JSON.stringify(dangerousEvent, null, 2));
  console.log('Decision:', dangerousDecision.decision);
  if (dangerousDecision.reason) {
    console.log('Reason:', dangerousDecision.reason);
  }
  console.log();

  // Example 3: Custom event for your application
  console.log('Example 3: Custom application event');
  const customEvent = {
    kind: 'database_query',
    operation: 'DELETE',
    table: 'users',
    where: { role: 'admin' },
    requestedBy: 'ai-agent-123',
  };

  const customDecision = await cupcake.evaluate(customEvent);
  console.log('Event:', JSON.stringify(customEvent, null, 2));
  console.log('Decision:', customDecision.decision);
  if (customDecision.reason) {
    console.log('Reason:', customDecision.reason);
  }
  console.log();

  // Example 4: Synchronous evaluation (blocks event loop)
  console.log('Example 4: Synchronous evaluation (CLI use case)');
  const syncEvent = {
    kind: 'file_write',
    path: '/etc/passwd',
    content: 'malicious content',
  };

  // Note: evaluateSync blocks, but is fine for CLI scripts
  const syncDecision = cupcake.evaluateSync(syncEvent);
  console.log('Decision:', syncDecision.decision);
  if (syncDecision.reason) {
    console.log('Reason:', syncDecision.reason);
  }

  console.log('\n✅ Examples complete!');
}

main().catch((error) => {
  console.error('Error:', error);
  process.exit(1);
});
