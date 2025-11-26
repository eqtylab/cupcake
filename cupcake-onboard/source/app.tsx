import React, {useState, useEffect} from 'react';
import {Box, Text, useInput} from 'ink';
import Spinner from 'ink-spinner';
import fs from 'fs';
import path from 'path';

// Color scheme: "Dark Espresso & Pastry"
const colors = {
	border: '#A06E5E',      // Baked Copper - box outlines
	text: '#8C827D',        // Warm Grey - standard info text
	green: '#4ADE80',       // Frosting Green - yes/success
	red: '#F87171',         // Berry Red - no/error
	link: '#60A5FA',        // Sky Blue - filenames
	select: '#4B5F6D',      // Slate Mousse - selected row bg
	action: '#FACC15',      // Lemon Curd - footer action
	white: '#FFFFFF',       // White - headers, selected text
	lavender: '#C4B5FD',    // Lavender - Claude Code branding
};

type RuleFile = {
	path: string;
	selected: boolean;
};

type PolicyRule = {
	id: string;
	name: string;
	description: string;
	enabled: boolean;
};

// Mock rules - will be generated from file analysis later
const defaultRules: PolicyRule[] = [
	{
		id: 'no-secrets',
		name: 'No Secrets',
		description: 'Block commits containing API keys or credentials',
		enabled: false,
	},
	{
		id: 'protected-paths',
		name: 'Protected Paths',
		description: 'Prevent modifications to critical config files',
		enabled: false,
	},
	{
		id: 'test-required',
		name: 'Test Required',
		description: 'Require tests before pushing to main branch',
		enabled: false,
	},
	{
		id: 'no-force-push',
		name: 'No Force Push',
		description: 'Block force pushes to protected branches',
		enabled: false,
	},
	{
		id: 'code-review',
		name: 'Code Review',
		description: 'Require approval before merging PRs',
		enabled: false,
	},
	{
		id: 'no-console-log',
		name: 'No Console Logs',
		description: 'Warn when console.log statements are added',
		enabled: false,
	},
	{
		id: 'max-file-size',
		name: 'Max File Size',
		description: 'Block files larger than 1MB from being committed',
		enabled: false,
	},
	{
		id: 'no-todo-comments',
		name: 'No TODO Comments',
		description: 'Warn when TODO comments are added without issue links',
		enabled: false,
	},
	{
		id: 'branch-naming',
		name: 'Branch Naming',
		description: 'Enforce branch naming conventions (feat/, fix/, etc.)',
		enabled: false,
	},
	{
		id: 'dependency-check',
		name: 'Dependency Check',
		description: 'Warn when adding new dependencies without approval',
		enabled: false,
	},
	{
		id: 'no-debugger',
		name: 'No Debugger',
		description: 'Block debugger statements from being committed',
		enabled: false,
	},
	{
		id: 'dockerfile-lint',
		name: 'Dockerfile Lint',
		description: 'Validate Dockerfile best practices',
		enabled: false,
	},
	{
		id: 'env-validation',
		name: 'Env Validation',
		description: 'Ensure .env.example is updated with new env vars',
		enabled: false,
	},
];

// Recursively scan directory for rule files
function scanForRuleFiles(dir: string, baseDir: string): string[] {
	const results: string[] = [];

	try {
		const entries = fs.readdirSync(dir, {withFileTypes: true});

		for (const entry of entries) {
			const fullPath = path.join(dir, entry.name);
			const relativePath = './' + path.relative(baseDir, fullPath);

			if (entry.isDirectory()) {
				// Skip node_modules and .git
				if (entry.name === 'node_modules' || entry.name === '.git') {
					continue;
				}
				// Recurse into subdirectories
				results.push(...scanForRuleFiles(fullPath, baseDir));
			} else if (entry.isFile()) {
				// Check for CLAUDE.md
				if (entry.name === 'CLAUDE.md') {
					results.push(relativePath);
				}
				// Check for .cursorrules
				if (entry.name === '.cursorrules') {
					results.push(relativePath);
				}
				// Check for .mdc files in .cursor/rules directories
				if (entry.name.endsWith('.mdc') && dir.includes('.cursor/rules')) {
					results.push(relativePath);
				}
			}
		}
	} catch {
		// Ignore permission errors
	}

	return results;
}

// Check if .cupcake folder exists
function checkCupcakeInit(dir: string): boolean {
	try {
		return fs.existsSync(path.join(dir, '.cupcake'));
	} catch {
		return false;
	}
}

type Props = {
	cwd?: string;
};

type AppState = 'selecting' | 'processing' | 'configuring' | 'generating' | 'complete';
type GeneratingPhase = 'creating' | 'testing' | 'editing' | 'retesting';
type TestStatus = 'pending' | 'passed' | 'revised' | 'editing' | 'retesting';

export default function App({cwd = process.cwd()}: Props) {
	const [files, setFiles] = useState<RuleFile[]>([]);
	const [cursor, setCursor] = useState(0);
	const [isScanning, setIsScanning] = useState(true);
	const [cupcakeInit, setCupcakeInit] = useState(false);
	const [appState, setAppState] = useState<AppState>('selecting');
	const [processingIndex, setProcessingIndex] = useState(0);
	const [rules, setRules] = useState<PolicyRule[]>(defaultRules);
	const [ruleCursor, setRuleCursor] = useState(0);
	const [generatingPhase, setGeneratingPhase] = useState<GeneratingPhase>('creating');
	const [generatingIndex, setGeneratingIndex] = useState(0);
	const [testResults, setTestResults] = useState<Map<string, TestStatus>>(new Map());
	const [revisedRules, setRevisedRules] = useState<string[]>([]);
	const [revisionIndex, setRevisionIndex] = useState(0);

	// Scan for files on mount
	useEffect(() => {
		// Check cupcake init status
		setCupcakeInit(checkCupcakeInit(cwd));

		// Scan for rule files
		const foundFiles = scanForRuleFiles(cwd, cwd);
		const ruleFiles: RuleFile[] = foundFiles.map(filePath => ({
			path: filePath,
			selected: false,  // Default to "no"
		}));

		setFiles(ruleFiles);
		setIsScanning(false);
	}, [cwd]);

	// Simulate processing animation
	useEffect(() => {
		if (appState !== 'processing') return;

		const selectedFiles = files.filter(f => f.selected);
		if (processingIndex >= selectedFiles.length) {
			setAppState('configuring');
			setRuleCursor(0);
			return;
		}

		const timer = setTimeout(() => {
			setProcessingIndex(prev => prev + 1);
		}, 1500); // 1.5 seconds per file

		return () => clearTimeout(timer);
	}, [appState, processingIndex, files]);

	// Simulate generating animation (creating policies, then testing, then revisions)
	useEffect(() => {
		if (appState !== 'generating') return;

		const enabledRules = rules.filter(r => r.enabled);

		if (generatingPhase === 'creating') {
			if (generatingIndex >= enabledRules.length) {
				// Move to testing phase
				setGeneratingPhase('testing');
				setGeneratingIndex(0);
				return;
			}

			const timer = setTimeout(() => {
				setGeneratingIndex(prev => prev + 1);
			}, 800); // 0.8 seconds per policy creation

			return () => clearTimeout(timer);
		} else if (generatingPhase === 'testing') {
			if (generatingIndex >= enabledRules.length) {
				// Check if any need revision
				const needsRevision = Array.from(testResults.entries())
					.filter(([_, status]) => status === 'revised')
					.map(([id]) => id);

				if (needsRevision.length > 0) {
					setRevisedRules(needsRevision);
					setRevisionIndex(0);
					setGeneratingPhase('editing');
				} else {
					setAppState('complete');
				}
				return;
			}

			const timer = setTimeout(() => {
				// Randomly assign "revised" (1/5 chance) or "passed"
				const currentRule = enabledRules[generatingIndex];
				if (currentRule) {
					const result = Math.random() < 0.2 ? 'revised' : 'passed';
					setTestResults(prev => new Map(prev).set(currentRule.id, result));
				}
				setGeneratingIndex(prev => prev + 1);
			}, 600); // 0.6 seconds per test

			return () => clearTimeout(timer);
		} else if (generatingPhase === 'editing') {
			if (revisionIndex >= revisedRules.length) {
				// Move to retesting
				setRevisionIndex(0);
				setGeneratingPhase('retesting');
				return;
			}

			const timer = setTimeout(() => {
				const ruleId = revisedRules[revisionIndex];
				if (ruleId) {
					setTestResults(prev => new Map(prev).set(ruleId, 'editing'));
				}
				setRevisionIndex(prev => prev + 1);
			}, 700); // 0.7 seconds per edit

			return () => clearTimeout(timer);
		} else if (generatingPhase === 'retesting') {
			if (revisionIndex >= revisedRules.length) {
				// Done - move to complete
				setAppState('complete');
				return;
			}

			const timer = setTimeout(() => {
				const ruleId = revisedRules[revisionIndex];
				if (ruleId) {
					setTestResults(prev => new Map(prev).set(ruleId, 'passed'));
				}
				setRevisionIndex(prev => prev + 1);
			}, 500); // 0.5 seconds per retest

			return () => clearTimeout(timer);
		}

		return undefined;
	}, [appState, generatingPhase, generatingIndex, rules, testResults, revisedRules, revisionIndex]);

	useInput((input, key) => {
		// Don't process input while scanning
		if (isScanning) return;

		// Handle ctrl+s to start processing (from selecting) or generate (from configuring)
		if (input === 's' && key.ctrl) {
			if (appState === 'selecting') {
				const selectedFiles = files.filter(f => f.selected);
				if (selectedFiles.length > 0) {
					setAppState('processing');
					setProcessingIndex(0);
				}
			} else if (appState === 'configuring') {
				const enabledRules = rules.filter(r => r.enabled);
				if (enabledRules.length > 0) {
					setAppState('generating');
					setGeneratingPhase('creating');
					setGeneratingIndex(0);
				}
			}
			return;
		}

		// Handle selecting state
		if (appState === 'selecting' && files.length > 0) {
			// Navigate up/down with wrapping (cycles from top to bottom and vice versa)
			if (key.upArrow) {
				setCursor(prev => (prev > 0 ? prev - 1 : files.length));
			}
			if (key.downArrow) {
				setCursor(prev => (prev < files.length ? prev + 1 : 0));
			}

			// Handle space or enter
			if (input === ' ' || key.return) {
				// Check if cursor is on the button (last position)
				if (cursor === files.length) {
					// Submit - start processing
					const selectedFiles = files.filter(f => f.selected);
					if (selectedFiles.length > 0) {
						setAppState('processing');
						setProcessingIndex(0);
					}
				} else {
					// Toggle file selection
					setFiles(prev =>
						prev.map((file, i) =>
							i === cursor ? {...file, selected: !file.selected} : file,
						),
					);
				}
			}
		}

		// Handle configuring state
		if (appState === 'configuring' && rules.length > 0) {
			// Navigate up/down with wrapping (cycles from top to bottom and vice versa)
			if (key.upArrow) {
				setRuleCursor(prev => (prev > 0 ? prev - 1 : rules.length));
			}
			if (key.downArrow) {
				setRuleCursor(prev => (prev < rules.length ? prev + 1 : 0));
			}

			// Handle space or enter
			if (input === ' ' || key.return) {
				// Check if cursor is on the button (last position)
				if (ruleCursor === rules.length) {
					// Submit - start generating
					const enabledRules = rules.filter(r => r.enabled);
					if (enabledRules.length > 0) {
						setAppState('generating');
						setGeneratingPhase('creating');
						setGeneratingIndex(0);
					}
				} else {
					// Toggle rule selection
					setRules(prev =>
						prev.map((rule, i) =>
							i === ruleCursor ? {...rule, enabled: !rule.enabled} : rule,
						),
					);
				}
			}
		}
	});

	// Get selected files for processing view
	const selectedFiles = files.filter(f => f.selected);

	// ASCII cupcake art
	const cupcakeArt = [
		"      |`-.__",
		"      /   _/",
		"     ****`",
		"    /    }",
		"   /  \\ /",
		"\\ /`   \\\\\\",
		" `\\    /_\\\\",
		" `~~~~~``~`",
	];

	return (
		<Box flexDirection="column" padding={1}>
			{/* Header with info box and cupcake art */}
			<Box alignItems="flex-end">
				<Box flexDirection="column">
					<Text bold color={colors.green}>
						Cupcake Onboarding
					</Text>
					<Text color={colors.text}>
						Convert your rules files into guarantees
					</Text>
					<Text> </Text>

					{/* Info Box */}
					<Box
						flexDirection="column"
						borderStyle="round"
						borderColor={colors.border}
						paddingX={1}
					>
						<Text>
							<Text color={colors.lavender}>agent: </Text>
							<Text color={colors.text}>Claude Code</Text>
						</Text>
						<Text>
							<Text color={colors.lavender}>cwd: </Text>
							<Text color={colors.text}>{cwd}</Text>
						</Text>
						<Text>
							<Text color={colors.lavender}>cupcake folder exists: </Text>
							<Text color={cupcakeInit ? colors.green : colors.red}>
								{cupcakeInit ? 'yes' : 'no, will create'}
							</Text>
						</Text>
						<Text> </Text>
						<Text>
							<Text color={colors.lavender}>rules files found: </Text>
							<Text color={colors.text}>
								{isScanning ? 'scanning...' : files.length.toString()}
							</Text>
						</Text>
					</Box>
				</Box>
				<Box flexGrow={1} justifyContent="center">
					<Box flexDirection="column">
						{cupcakeArt.map((line, i) => (
							<Text key={i} color={colors.border}>{line}</Text>
						))}
					</Box>
				</Box>
			</Box>

			<Text> </Text>

			{/* Main content area - changes based on app state */}
			{appState === 'selecting' ? (
				<>
					{/* File Selector Box */}
					<Box
						flexDirection="column"
						borderStyle="round"
						borderColor={colors.border}
						paddingX={1}
					>
						{/* Header row */}
						<Box justifyContent="space-between">
							<Box>
								<Box width={4}>
									<Text> </Text>
								</Box>
								<Text bold color={colors.white}>FOUND</Text>
							</Box>
							<Text bold color={colors.white}>USE</Text>
						</Box>

						{/* File rows */}
						{isScanning ? (
							<Text color={colors.text}>Scanning directories...</Text>
						) : files.length === 0 ? (
							<Text color={colors.text}>No rule files found</Text>
						) : (
							<>
								{files.map((file, index) => {
									const isHighlighted = index === cursor;
									return (
										<Box key={file.path} justifyContent="space-between">
											<Box>
												<Box width={4}>
													<Text color={colors.white}>{isHighlighted ? '>>' : '  '}</Text>
												</Box>
												<Text
													color={isHighlighted ? colors.white : colors.link}
													backgroundColor={isHighlighted ? colors.select : undefined}
												>
													{file.path}
												</Text>
											</Box>
											<Text
												color={file.selected ? colors.green : colors.red}
												backgroundColor={isHighlighted ? colors.select : undefined}
											>
												{file.selected ? 'yes' : 'no'}
											</Text>
										</Box>
									);
								})}

								{/* Submit button */}
								<Text> </Text>
								<Box justifyContent="center">
									<Text
										color={cursor === files.length ? colors.white : colors.action}
										backgroundColor={cursor === files.length ? colors.select : undefined}
									>
										{cursor === files.length ? '>> ' : '   '}
										[ Continue → ]
									</Text>
								</Box>
							</>
						)}
					</Box>

					<Text> </Text>

					{/* Footer */}
					<Text color={colors.action}>To begin: Enter or ctrl + s</Text>
					<Text color={colors.text}>To quit: ctrl + c</Text>
				</>
			) : appState === 'processing' ? (
				<>
					{/* Processing View */}
					<Box
						flexDirection="column"
						borderStyle="round"
						borderColor={colors.border}
						paddingX={1}
					>
						<Text bold color={colors.white}>Processing selected files...</Text>
						<Text> </Text>
						{selectedFiles.map((file, index) => (
							<Box key={file.path}>
								<Box width={3}>
									{index < processingIndex ? (
										<Text color={colors.green}>✓</Text>
									) : index === processingIndex ? (
										<Text color={colors.action}>
											<Spinner type="dots" />
										</Text>
									) : (
										<Text color={colors.text}>○</Text>
									)}
								</Box>
								<Text
									color={
										index < processingIndex
											? colors.green
											: index === processingIndex
											? colors.white
											: colors.text
									}
								>
									{file.path}
								</Text>
							</Box>
						))}
					</Box>

					<Text> </Text>
					<Text color={colors.text}>Processing {processingIndex + 1} of {selectedFiles.length}...</Text>
				</>
			) : appState === 'configuring' ? (
				<>
					{/* Configuring View - Rules Table */}
					<Box
						flexDirection="column"
						borderStyle="round"
						borderColor={colors.border}
						paddingX={1}
					>
						<Text bold color={colors.white}>Select policies to enable:</Text>
						<Text> </Text>

						{/* Header row */}
						<Box justifyContent="space-between">
							<Box>
								<Box width={4}>
									<Text> </Text>
								</Box>
								<Box width={20}>
									<Text bold color={colors.white}>RULE</Text>
								</Box>
								<Text bold color={colors.white}>DESCRIPTION</Text>
							</Box>
							<Text bold color={colors.white}>USE</Text>
						</Box>

						{/* Rule rows */}
						{rules.map((rule, index) => {
							const isHighlighted = index === ruleCursor;
							return (
								<Box key={rule.id} justifyContent="space-between">
									<Box>
										<Box width={4}>
											<Text color={colors.white}>{isHighlighted ? '>>' : '  '}</Text>
										</Box>
										<Box width={20}>
											<Text
												color={isHighlighted ? colors.white : colors.link}
												backgroundColor={isHighlighted ? colors.select : undefined}
											>
												{rule.name}
											</Text>
										</Box>
										<Text
											color={isHighlighted ? colors.white : colors.text}
											backgroundColor={isHighlighted ? colors.select : undefined}
										>
											{rule.description}
										</Text>
									</Box>
									<Text
										color={rule.enabled ? colors.green : colors.red}
										backgroundColor={isHighlighted ? colors.select : undefined}
									>
										{rule.enabled ? 'yes' : 'no'}
									</Text>
								</Box>
							);
						})}

						{/* Submit button */}
						<Text> </Text>
						<Box justifyContent="center">
							<Text
								color={ruleCursor === rules.length ? colors.white : colors.action}
								backgroundColor={ruleCursor === rules.length ? colors.select : undefined}
							>
								{ruleCursor === rules.length ? '>> ' : '   '}
								[ Generate Policies → ]
							</Text>
						</Box>
					</Box>

					<Text> </Text>
					<Text color={colors.action}>To finish: Enter or ctrl + s</Text>
					<Text color={colors.text}>To quit: ctrl + c</Text>
				</>
			) : appState === 'generating' ? (
				<>
					{/* Generating View */}
					<Box
						flexDirection="column"
						borderStyle="round"
						borderColor={colors.border}
						paddingX={1}
					>
						{generatingPhase === 'creating' ? (
							<>
								<Text bold color={colors.white}>Creating Rego policies...</Text>
								<Text> </Text>
								{rules.filter(r => r.enabled).map((rule, index) => (
									<Box key={rule.id}>
										<Box width={3}>
											{index < generatingIndex ? (
												<Text color={colors.green}>✓</Text>
											) : index === generatingIndex ? (
												<Text color={colors.action}>
													<Spinner type="dots" />
												</Text>
											) : (
												<Text color={colors.text}>○</Text>
											)}
										</Box>
										<Text
											color={
												index < generatingIndex
													? colors.green
													: index === generatingIndex
													? colors.white
													: colors.text
											}
										>
											.cupcake/policies/{rule.id}.rego
										</Text>
									</Box>
								))}
							</>
						) : (
							<>
								<Text bold color={colors.white}>
									{generatingPhase === 'testing' && 'Testing & validating policies...'}
									{generatingPhase === 'editing' && 'Revising policies...'}
									{generatingPhase === 'retesting' && 'Re-testing revised policies...'}
								</Text>
								<Text> </Text>
								{rules.filter(r => r.enabled).map((rule, index) => {
									const result = testResults.get(rule.id);
									const isInRevisionList = revisedRules.includes(rule.id);
									const revisionIdx = revisedRules.indexOf(rule.id);

									// Determine icon and status text
									let icon = '○';
									let iconColor = colors.text;
									let textColor = colors.text;
									let statusText = 'pending';

									if (generatingPhase === 'testing') {
										if (index < generatingIndex) {
											if (result === 'revised') {
												icon = '↻';
												iconColor = colors.action;
												textColor = colors.action;
												statusText = 'needs revision';
											} else {
												icon = '✓';
												iconColor = colors.green;
												textColor = colors.green;
												statusText = 'passed';
											}
										} else if (index === generatingIndex) {
											icon = '';
											iconColor = colors.action;
											textColor = colors.white;
											statusText = 'testing...';
										}
									} else if (generatingPhase === 'editing' || generatingPhase === 'retesting') {
										if (result === 'passed') {
											icon = '✓';
											iconColor = colors.green;
											textColor = colors.green;
											statusText = 'passed';
										} else if (isInRevisionList) {
											if (generatingPhase === 'editing') {
												if (revisionIdx < revisionIndex) {
													icon = '✎';
													iconColor = colors.action;
													textColor = colors.action;
													statusText = 'edited';
												} else if (revisionIdx === revisionIndex) {
													icon = '';
													iconColor = colors.action;
													textColor = colors.white;
													statusText = 'editing...';
												} else {
													icon = '↻';
													iconColor = colors.action;
													textColor = colors.action;
													statusText = 'needs revision';
												}
											} else if (generatingPhase === 'retesting') {
												if (revisionIdx < revisionIndex) {
													icon = '✓';
													iconColor = colors.green;
													textColor = colors.green;
													statusText = 'passed';
												} else if (revisionIdx === revisionIndex) {
													icon = '';
													iconColor = colors.action;
													textColor = colors.white;
													statusText = 're-testing...';
												} else {
													icon = '✎';
													iconColor = colors.action;
													textColor = colors.action;
													statusText = 'edited';
												}
											}
										}
									}

									return (
										<Box key={rule.id}>
											<Box width={3}>
												{icon === '' ? (
													<Text color={iconColor}>
														<Spinner type="dots" />
													</Text>
												) : (
													<Text color={iconColor}>{icon}</Text>
												)}
											</Box>
											<Text color={textColor}>
												{rule.name} - {statusText}
											</Text>
										</Box>
									);
								})}
							</>
						)}
					</Box>

					<Text> </Text>
					<Text color={colors.text}>
						{generatingPhase === 'creating' &&
							`Creating policy ${Math.min(generatingIndex + 1, rules.filter(r => r.enabled).length)} of ${rules.filter(r => r.enabled).length}...`
						}
						{generatingPhase === 'testing' &&
							`Testing policy ${Math.min(generatingIndex + 1, rules.filter(r => r.enabled).length)} of ${rules.filter(r => r.enabled).length}...`
						}
						{generatingPhase === 'editing' &&
							`Editing policy ${Math.min(revisionIndex + 1, revisedRules.length)} of ${revisedRules.length}...`
						}
						{generatingPhase === 'retesting' &&
							`Re-testing policy ${Math.min(revisionIndex + 1, revisedRules.length)} of ${revisedRules.length}...`
						}
					</Text>
				</>
			) : (
				<>
					{/* Complete View - Final Report */}
					<Box
						flexDirection="column"
						borderStyle="round"
						borderColor={colors.border}
						paddingX={1}
					>
						<Text bold color={colors.green}>Setup Complete!</Text>
						<Text> </Text>
						<Text color={colors.white}>Summary:</Text>
						<Text color={colors.text}>  Files analyzed: {selectedFiles.length}</Text>
						<Text color={colors.text}>  Policies created: {rules.filter(r => r.enabled).length}</Text>
						<Text> </Text>
						<Text color={colors.white}>Policies generated:</Text>
						{rules.filter(r => r.enabled).map((rule) => (
							<Text key={rule.id} color={colors.green}>
								  ✓ .cupcake/policies/{rule.id}.rego
							</Text>
						))}
						<Text> </Text>
						<Text color={colors.text}>
							Run <Text color={colors.link}>cupcake eval</Text> to test your policies
						</Text>
					</Box>

					<Text> </Text>
					<Text color={colors.action}>Cupcake is ready!</Text>
					<Text color={colors.text}>To quit: ctrl + c</Text>
				</>
			)}
		</Box>
	);
}
