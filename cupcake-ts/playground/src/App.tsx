import React, { useState, useCallback, useRef } from 'react';
import Editor, { type Monaco, type OnMount } from '@monaco-editor/react';
import type { editor } from 'monaco-editor';
import * as policySDK from '@cupcake/policy';
import Button from './components/Button';
// Import the actual SDK type definitions as raw text
import sdkTypes from '../../policy/dist/index.d.ts?raw';

interface CompilationState {
  isCompiling: boolean;
  error: string | null;
  lastCompiledAt: Date | null;
}

const SDK_URI = 'file:///node_modules/%40cupcake/policy/index.d.ts';
const USER_URI = 'file:///policy.ts';

function handleEditorWillMount(monaco: Monaco) {
  // Configure compiler options with path mapping
  monaco.languages.typescript.typescriptDefaults.setCompilerOptions({
    target: monaco.languages.typescript.ScriptTarget.ES2020,
    allowNonTsExtensions: true,
    moduleResolution: monaco.languages.typescript.ModuleResolutionKind.NodeJs,
    module: monaco.languages.typescript.ModuleKind.ESNext,
    noEmit: true,
    strict: true,
    baseUrl: 'file:///',
    paths: {
      '@cupcake/policy': ['node_modules/@cupcake/policy/index.d.ts'],
    },
  });

  // Create SDK types model (read-only)
  const sdkUri = monaco.Uri.parse(SDK_URI);
  if (!monaco.editor.getModel(sdkUri)) {
    monaco.editor.createModel(sdkTypes, 'typescript', sdkUri);
  }

  // Also add as extra lib for module resolution
  monaco.languages.typescript.typescriptDefaults.addExtraLib(sdkTypes, sdkUri.toString());
}

const INITIAL_CODE = `import { policy, cant, canOnly, addContext } from '@cupcake/policy';

// Define what a technical writer can and cannot do
const techWriterPolicy = policy('technical writer',
  // Cannot write to source code
  cant('write to source code', ['Write', 'Edit'])
    .severity('HIGH')
    .when(({ path }) => [path.contains('src/')]),

  // Cannot push to main
  cant('push to main branch', 'Bash')
    .when(({ command }) => [
      command.contains('git push'),
      command.contains('main'),
    ]),

  // Can only access documentation files
  canOnly('access documentation', ['Read', 'Write', 'Edit'])
    .when(({ path }) => [
      path.endsWith('.md'),
    ]),

  // Add helpful context
  addContext('Follow the company style guide for all documentation.'),
);

export default techWriterPolicy;
`;

const TEMPLATES = [
  {
    name: 'Technical Writer',
    description: 'Restrict access to source code, allow docs',
    code: INITIAL_CODE,
  },
  {
    name: 'Security Policy',
    description: 'Block sensitive operations',
    code: `import { policy, cant, mustAsk } from '@cupcake/policy';

const securityPolicy = policy('security restrictions',
  // Block all rm -rf commands
  cant('delete recursively', 'Bash')
    .severity('CRITICAL')
    .when(({ command }) => [
      command.contains('rm -rf'),
    ]),

  // Block access to secrets
  cant('access secrets', 'Read')
    .when(({ path }) => [
      path.contains('.env'),
    ]),

  // Must ask before modifying configs
  mustAsk('before changing configs', 'Edit')
    .when(({ path }) => [
      path.endsWith('.json'),
      path.contains('config'),
    ])
    .reason('Configuration changes require approval')
    .question('This modifies a config file. Proceed?'),
);

export default securityPolicy;
`,
  },
  {
    name: 'Minimal Example',
    description: 'Simplest possible policy',
    code: `import { policy, cant } from '@cupcake/policy';

const minimalPolicy = policy('minimal',
  cant('run dangerous commands', 'Bash')
    .when(({ command }) => [
      command.contains('sudo'),
    ]),
);

export default minimalPolicy;
`,
  },
];

const App: React.FC = () => {
  const [tsCode, setTsCode] = useState<string>(INITIAL_CODE);
  const [regoCode, setRegoCode] = useState<string>('');
  const [compilation, setCompilation] = useState<CompilationState>({
    isCompiling: false,
    error: null,
    lastCompiledAt: null,
  });
  const [showTemplates, setShowTemplates] = useState(false);
  const [viewingSDK, setViewingSDK] = useState(false);
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const monacoRef = useRef<Monaco | null>(null);
  const userModelRef = useRef<editor.ITextModel | null>(null);

  // Handle editor mount - set up go-to-definition and track state
  const handleEditorDidMount: OnMount = useCallback((editor, monaco) => {
    editorRef.current = editor;
    monacoRef.current = monaco;

    // Create/get user model
    const userUri = monaco.Uri.parse(USER_URI);
    let userModel = monaco.editor.getModel(userUri);
    if (!userModel) {
      userModel = monaco.editor.createModel(tsCode, 'typescript', userUri);
    }
    userModelRef.current = userModel;
    editor.setModel(userModel);

    // Access the internal code editor service for go-to-definition
    const editorService = (editor as any)._codeEditorService;
    if (!editorService) return;

    const openEditorBase = editorService.openCodeEditor.bind(editorService);

    // Override to handle opening external models
    editorService.openCodeEditor = async (input: any, source: any) => {
      const result = await openEditorBase(input, source);
      if (result === null) {
        const targetModel = monaco.editor.getModel(input.resource);
        if (targetModel) {
          const isSDK = input.resource.toString() === SDK_URI;
          if (isSDK) {
            setViewingSDK(true);
          }
          source.setModel(targetModel);
          if (input.options?.selection) {
            source.setSelection(input.options.selection);
            source.revealLineInCenter(input.options.selection.startLineNumber);
          }
        }
      }
      return result;
    };
  }, [tsCode]);

  // Go back to user's code
  const handleBackToEditor = useCallback(() => {
    if (editorRef.current && userModelRef.current) {
      editorRef.current.setModel(userModelRef.current);
      editorRef.current.updateOptions({ readOnly: false });
      setViewingSDK(false);
    }
  }, []);

  const handleCompile = useCallback(() => {
    setCompilation((prev) => ({ ...prev, isCompiling: true, error: null }));

    try {
      // Transform the code: strip imports, convert export default to return
      let code = tsCode
        // Remove import statements
        .replace(/^import\s+.*?from\s+['"]@cupcake\/policy['"];?\s*$/gm, '')
        // Convert "export default X" to "return X"
        .replace(/^export\s+default\s+/gm, 'return ');

      // Create evaluation context with all SDK functions injected
      const evalFunc = new Function(
        'policy',
        'cant',
        'canOnly',
        'addContext',
        'mustHalt',
        'mustAsk',
        'mustModify',
        'mustBlock',
        'reason',
        'defineSignal',
        'defineTypedSignal',
        'defineConstant',
        `
        "use strict";
        ${code}
        `
      );

      // Execute with SDK functions
      const policyObj = evalFunc(
        policySDK.policy,
        policySDK.cant,
        policySDK.canOnly,
        policySDK.addContext,
        policySDK.mustHalt,
        policySDK.mustAsk,
        policySDK.mustModify,
        policySDK.mustBlock,
        policySDK.reason,
        policySDK.defineSignal,
        policySDK.defineTypedSignal,
        policySDK.defineConstant
      );

      if (!policyObj || typeof policyObj !== 'object') {
        throw new Error('Use "export default yourPolicy" to specify which policy to compile.');
      }

      // Compile to Rego
      const rego = policySDK.compile(policyObj);
      setRegoCode(rego);
      setCompilation({
        isCompiling: false,
        error: null,
        lastCompiledAt: new Date(),
      });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : 'Unknown error';
      setCompilation({
        isCompiling: false,
        error: message,
        lastCompiledAt: null,
      });
      setRegoCode('');
    }
  }, [tsCode]);

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  const applyTemplate = (code: string) => {
    setTsCode(code);
    setShowTemplates(false);
  };

  return (
    <div className="flex flex-col h-screen w-screen bg-[#09090b] text-zinc-100 overflow-hidden">
      {/* Header */}
      <header className="h-14 border-b border-zinc-800 flex items-center justify-between px-6 shrink-0 bg-[#09090b]/80 backdrop-blur-sm z-50">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 bg-gradient-to-br from-pink-500 to-violet-500 rounded-lg flex items-center justify-center">
            <span className="text-white text-lg">C</span>
          </div>
          <div>
            <h1 className="text-sm font-semibold tracking-tight">
              Cupcake <span className="text-zinc-500 font-normal">Playground</span>
            </h1>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <div className="relative">
            <Button variant="secondary" onClick={() => setShowTemplates(!showTemplates)}>
              Templates
              <svg
                className={`w-4 h-4 transition-transform ${showTemplates ? 'rotate-180' : ''}`}
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
              </svg>
            </Button>

            {showTemplates && (
              <div className="absolute right-0 mt-2 w-64 bg-zinc-900 border border-zinc-800 rounded-lg shadow-2xl z-50 overflow-hidden">
                {TEMPLATES.map((t) => (
                  <button
                    key={t.name}
                    onClick={() => applyTemplate(t.code)}
                    className="w-full text-left px-4 py-3 hover:bg-zinc-800 transition-colors border-b border-zinc-800 last:border-0"
                  >
                    <div className="text-sm font-medium">{t.name}</div>
                    <div className="text-xs text-zinc-500 mt-0.5">{t.description}</div>
                  </button>
                ))}
              </div>
            )}
          </div>
          <Button onClick={handleCompile} isLoading={compilation.isCompiling} className="w-28">
            Compile
          </Button>
        </div>
      </header>

      {/* Main Content */}
      <main className="flex-1 flex overflow-hidden">
        {/* Left Side: TypeScript */}
        <div className="flex-1 flex flex-col border-r border-zinc-800">
          <div className="h-10 bg-zinc-900/50 flex items-center px-4 justify-between border-b border-zinc-800">
            <div className="flex items-center gap-2">
              {viewingSDK ? (
                <>
                  <button
                    onClick={handleBackToEditor}
                    className="flex items-center gap-1 text-xs text-zinc-400 hover:text-zinc-200 transition-colors"
                  >
                    <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
                    </svg>
                    Back
                  </button>
                  <span className="text-zinc-600">|</span>
                  <span className="text-xs font-medium text-zinc-400 uppercase tracking-wider">SDK Types</span>
                  <span className="px-1.5 py-0.5 bg-zinc-800 rounded text-[10px] font-mono text-amber-400">
                    Read Only
                  </span>
                </>
              ) : (
                <>
                  <span className="text-xs font-medium text-zinc-400 uppercase tracking-wider">Source</span>
                  <span className="px-1.5 py-0.5 bg-zinc-800 rounded text-[10px] font-mono text-blue-400">
                    TypeScript
                  </span>
                </>
              )}
            </div>
          </div>
          <div className="flex-1">
            <Editor
              height="100%"
              defaultLanguage="typescript"
              theme="vs-dark"
              onChange={(value) => {
                if (!viewingSDK) {
                  setTsCode(value || '');
                }
              }}
              beforeMount={handleEditorWillMount}
              onMount={handleEditorDidMount}
              options={{
                minimap: { enabled: false },
                fontSize: 13,
                fontFamily: 'JetBrains Mono, monospace',
                scrollBeyondLastLine: false,
                lineNumbers: 'on',
                glyphMargin: false,
                folding: true,
                padding: { top: 16 },
                scrollbar: {
                  vertical: 'auto',
                  horizontal: 'auto',
                },
                readOnly: viewingSDK,
              }}
            />
          </div>
        </div>

        {/* Right Side: Rego */}
        <div className="flex-1 flex flex-col bg-zinc-950">
          <div className="h-10 bg-zinc-900/50 flex items-center px-4 justify-between border-b border-zinc-800">
            <div className="flex items-center gap-2">
              <span className="text-xs font-medium text-zinc-400 uppercase tracking-wider">Output</span>
              <span className="px-1.5 py-0.5 bg-zinc-800 rounded text-[10px] font-mono text-emerald-400">
                Rego
              </span>
            </div>
            {regoCode && (
              <button
                onClick={() => copyToClipboard(regoCode)}
                className="text-[10px] font-medium text-zinc-500 hover:text-zinc-300 transition-colors flex items-center gap-1"
              >
                <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3"
                  />
                </svg>
                Copy
              </button>
            )}
          </div>
          <div className="flex-1 relative">
            {!regoCode && !compilation.isCompiling && !compilation.error && (
              <div className="absolute inset-0 flex flex-col items-center justify-center text-zinc-600 gap-4">
                <div className="p-4 rounded-full bg-zinc-900 border border-zinc-800">
                  <svg className="w-8 h-8 opacity-30" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={1}
                      d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"
                    />
                  </svg>
                </div>
                <p className="text-sm font-light">Click Compile to generate Rego</p>
              </div>
            )}

            {compilation.isCompiling && (
              <div className="absolute inset-0 z-10 bg-zinc-950/50 backdrop-blur-[2px] flex items-center justify-center">
                <div className="flex flex-col items-center gap-3">
                  <div className="w-5 h-5 border-2 border-indigo-500 border-t-transparent rounded-full animate-spin" />
                  <span className="text-xs font-medium text-indigo-400">Compiling...</span>
                </div>
              </div>
            )}

            <Editor
              height="100%"
              defaultLanguage="rego"
              theme="vs-dark"
              value={regoCode}
              options={{
                readOnly: true,
                minimap: { enabled: false },
                fontSize: 13,
                fontFamily: 'JetBrains Mono, monospace',
                scrollBeyondLastLine: false,
                lineNumbers: 'on',
                glyphMargin: false,
                folding: true,
                padding: { top: 16 },
                scrollbar: {
                  vertical: 'auto',
                  horizontal: 'auto',
                },
              }}
            />
          </div>
        </div>
      </main>

      {/* Footer Status Bar */}
      <footer className="h-8 border-t border-zinc-800 bg-[#09090b] flex items-center justify-between px-4 shrink-0 text-[10px] text-zinc-500 font-medium">
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-1.5">
            <div className="w-1.5 h-1.5 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]" />
            <span>Ready</span>
          </div>
          {compilation.lastCompiledAt && (
            <span>Last compiled: {compilation.lastCompiledAt.toLocaleTimeString()}</span>
          )}
        </div>

        <div className="flex items-center gap-3">
          {compilation.error && (
            <span className="text-rose-400 flex items-center gap-1 max-w-md truncate">
              <svg className="w-3 h-3 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              {compilation.error}
            </span>
          )}
          <span className="text-zinc-600">@cupcake/policy</span>
        </div>
      </footer>
    </div>
  );
};

export default App;
