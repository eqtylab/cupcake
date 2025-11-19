#!/usr/bin/env node
import React from 'react';
import {render} from 'ink';
import meow from 'meow';
import App from './app.js';

const cli = meow(
	`
	Usage
	  $ cupcake-onboard

	Options
		--cwd  Working directory (defaults to current)

	Examples
	  $ cupcake-onboard
	  $ cupcake-onboard --cwd=/path/to/project
`,
	{
		importMeta: import.meta,
		flags: {
			cwd: {
				type: 'string',
			},
		},
	},
);

render(<App cwd={cli.flags.cwd} />);
