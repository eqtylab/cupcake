import { policy, addContext, cant, canOnly, reason, defineSignal } from '../src/index';

console.log('Running examples...');

console.log('============= PIRATE MODE =============');

const pirateMode = policy(
  'pirate mode',
  addContext('Always talk like a pirate. Use "arr", "matey", and "avast" in responses.').when(({ hookEventName }) => [
    hookEventName.equals('UserPromptSubmit'),
  ]),
);

console.log(JSON.stringify(pirateMode, null, 2));

console.log();

console.log('============= SYMLINK DETECTION =============');
const dangerousDirectories = ['/etc/', '/bin/', '/sbin/', '/usr/', '/System/', '/Library/', '.cupcake/', '.git/'];

const symlinkDetection = policy(
  'symlink detection',
  cant('symlink to dangerous location', ['Write', 'Edit', 'Read'])
    .severity('HIGH')
    .ruleId('SYMLINK-001')
    .when(({ isSymlink, resolvedFilePath }) => [
      isSymlink.equals(true),
      resolvedFilePath.lower().containsAny(dangerousDirectories),
    ])
    .reason(
      ({ originalFilePath, resolvedFilePath }) =>
        reason`Symlink detected: '${originalFilePath}' resolves to protected location '${resolvedFilePath}'`,
    ),
);

console.log(JSON.stringify(symlinkDetection, null, 2));

console.log();
console.log('============= TECHNICAL WRITER =============');

const simulateGitBranch = (): Promise<string> => {
  return Promise.resolve('main');
};

const gitBranch = defineSignal('gitBranch', async () => {
  const result = await simulateGitBranch();
  return result.trim();
});

const techWriterPolicy = policy(
  'technical writer',
  cant('write to src', ['Write', 'Edit', 'Bash']).when(({ path }) => [path.contains('src/')]),
  cant('push to main', 'Bash').when(({ command }) => [command.contains('git push'), gitBranch.equals('main')]),
  addContext('This is how we write our blogs...'),
);

console.log(JSON.stringify(techWriterPolicy, null, 2));
console.log();

console.log();
console.log('============= TECHNICAL WRITER (canOnly) =============');

const techWriterOnlyPolicy = policy(
  'technical writer',
  canOnly('access blog files', ['Read', 'Write']).when(({ path }) => [path.contains('src/blog')]),
  canOnly('push to origin', 'Bash').when(({ command }) => [command.equals('git push origin')]),
);

console.log(JSON.stringify(techWriterOnlyPolicy, null, 2));
console.log();
