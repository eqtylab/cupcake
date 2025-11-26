/**
 * Express server with Cupcake policy enforcement
 *
 * This example shows how to use Cupcake in a web server to enforce
 * policies on AI agent actions before execution.
 */

import express from 'express';
import { init, evaluate, version, CupcakeError } from '../index';

const app = express();
app.use(express.json());

// Middleware to enforce Cupcake policies
async function policyMiddleware(req: express.Request, res: express.Response, next: express.NextFunction) {
  // Skip policy check for health endpoint
  if (req.path === '/health') {
    return next();
  }

  try {
    // Extract action from request (your application defines this structure)
    const action = req.body;

    // Evaluate the action against policies
    const decision = await evaluate(action);

    // Log the decision
    console.log(`Policy decision for ${req.path}:`, decision.decision);

    // Attach decision to request for downstream handlers
    (req as any).policyDecision = decision;

    // Block requests that violate policies
    if (decision.decision === 'Halt' || decision.decision === 'Deny') {
      return res.status(403).json({
        error: 'Policy violation',
        decision: decision.decision,
        reason: decision.reason,
        rule_id: decision.rule_id,
      });
    }

    // For 'Ask' decisions, require human approval
    if (decision.decision === 'Ask') {
      return res.status(202).json({
        message: 'Action requires approval',
        question: decision.question,
        reason: decision.reason,
        approval_required: true,
      });
    }

    // Allow the request to proceed
    next();
  } catch (error) {
    console.error('Policy evaluation error:', error);

    if (error instanceof CupcakeError) {
      return res.status(500).json({
        error: 'Policy evaluation failed',
        code: error.code,
        message: error.message,
      });
    }

    return res.status(500).json({ error: 'Internal server error' });
  }
}

// Apply policy middleware to all routes
app.use(policyMiddleware);

// Health check (bypasses policy)
app.get('/health', (req, res) => {
  res.json({ status: 'ok', cupcake_version: version() });
});

// Execute a shell command (policy enforced)
app.post('/execute/shell', (req, res) => {
  const { command, args } = req.body;
  const decision = (req as any).policyDecision;

  // Policy allowed this action
  res.json({
    status: 'allowed',
    command,
    args,
    decision: decision.decision,
    context: decision.context, // Additional guidance from policy
  });

  // TODO: Actually execute the command here (not shown for safety)
});

// Execute a database query (policy enforced)
app.post('/execute/database', (req, res) => {
  const { query, parameters } = req.body;
  const decision = (req as any).policyDecision;

  res.json({
    status: 'allowed',
    query,
    parameters,
    decision: decision.decision,
    context: decision.context,
  });

  // TODO: Actually execute the query here
});

// API call (policy enforced)
app.post('/execute/api', (req, res) => {
  const { endpoint, method, body } = req.body;
  const decision = (req as any).policyDecision;

  res.json({
    status: 'allowed',
    endpoint,
    method,
    decision: decision.decision,
    context: decision.context,
  });

  // TODO: Actually make the API call here
});

// Start server
async function startServer() {
  try {
    // Initialize Cupcake at server startup
    console.log('Initializing Cupcake...');
    await init('.cupcake', 'claude');
    console.log(`✓ Cupcake ${version()} initialized`);

    const PORT = process.env.PORT || 3000;
    app.listen(PORT, () => {
      console.log(`\n🧁 Server running on port ${PORT}`);
      console.log('Policy enforcement is active on all routes except /health');
      console.log('\nExample requests:');
      console.log('  curl -X POST http://localhost:3000/execute/shell \\');
      console.log('    -H "Content-Type: application/json" \\');
      console.log('    -d \'{"kind":"shell","command":"ls","args":["-la"]}\'');
      console.log();
    });
  } catch (error) {
    console.error('Failed to start server:', error);
    process.exit(1);
  }
}

startServer();
