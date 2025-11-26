/**
 * Build script for Cupcake OpenCode plugin
 * 
 * Bundles all TypeScript files into a single cupcake.js that OpenCode can load
 * from .opencode/plugin/cupcake.js
 * 
 * IMPORTANT: OpenCode loads all exports from plugin files and tries to call them.
 * We only export the CupcakePlugin function to avoid errors.
 */
import * as esbuild from 'esbuild';
import { writeFileSync, readFileSync } from 'fs';
import { unlink } from 'fs/promises';

async function build() {
  try {
    // Bundle all source files into a single JS file
    await esbuild.build({
      entryPoints: ['src/index.ts'],
      bundle: true,
      outfile: 'dist/cupcake.tmp.js',
      format: 'esm',
      platform: 'node',
      target: 'es2022',
      sourcemap: false,
      // Mark @opencode-ai/plugin as external (provided by OpenCode)
      external: ['@opencode-ai/plugin'],
    });

    // Read the bundled file and modify exports to only export CupcakePlugin
    let bundledCode = readFileSync('dist/cupcake.tmp.js', 'utf-8');
    
    // Remove the multi-export line and replace with just CupcakePlugin
    bundledCode = bundledCode.replace(
      /export \{[\s\S]*?\};/,
      'export { CupcakePlugin };'
    );
    
    // Add banner
    const banner = `/**
 * Cupcake OpenCode Plugin
 * 
 * Install: Copy this file to .opencode/plugin/cupcake.js
 * 
 * This plugin integrates Cupcake policy enforcement with OpenCode.
 * It intercepts tool executions and evaluates them against your policies.
 */

`;
    
    writeFileSync('dist/cupcake.js', banner + bundledCode);
    console.log('✅ Built dist/cupcake.js (single export)');

    // Clean up temp file
    await unlink('dist/cupcake.tmp.js');

    // Also generate a simple install script
    const installInstructions = `# Installation

Copy cupcake.js to your project:

\`\`\`bash
mkdir -p .opencode/plugin
cp cupcake.js .opencode/plugin/
\`\`\`

Or install globally:

\`\`\`bash
mkdir -p ~/.config/opencode/plugin
cp cupcake.js ~/.config/opencode/plugin/
\`\`\`
`;

    writeFileSync('dist/INSTALL.md', installInstructions);
    console.log('✅ Generated dist/INSTALL.md');

  } catch (error) {
    console.error('Build failed:', error);
    process.exit(1);
  }
}

build();
