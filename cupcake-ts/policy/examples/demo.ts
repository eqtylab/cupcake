/**
 * Demo: Playground Evolved API
 */

import { policy, cant, canOnly, addContext, defineSignal, compile } from '../src/index';

// Define a signal for git branch
const gitBranch = defineSignal('gitBranch', async () => 'main');

// Create a policy
const techWriterPolicy = policy(
  'technical writer',

  // Deny writing to src directory
  cant('write to src', ['Write', 'Edit', 'Bash'])
    .severity('HIGH')
    .ruleId('TW-001')
    .when(({ path }) => [path.contains('src/')]),

  // Deny pushing to main
  cant('push to main', 'Bash')
    .severity('CRITICAL')
    .ruleId('TW-002')
    .when(({ command }) => [
      command.contains('git push'),
      gitBranch.equals('main'),
    ]),

  // Only allow reading blog files
  canOnly('access blog files', ['Read', 'Write'])
    .when(({ path }) => [path.contains('blog/')]),

  // Always inject this context
  addContext('Follow the company style guide for all content.'),

  // Conditional context
  addContext('Remember to use proper headings and formatting.')
    .when(({ hookEventName }) => [hookEventName.equals('UserPromptSubmit')]),
);

console.log('Policy created:', techWriterPolicy.name);
console.log('Rules:', techWriterPolicy.rules.length);
console.log('Has allow rules:', techWriterPolicy.hasAllowRules);
console.log('\nCompiled output:');
console.log(compile(techWriterPolicy));
