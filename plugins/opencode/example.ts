/**
 * Example: How to use the Cupcake plugin in an OpenCode plugin
 *
 * This file demonstrates the plugin structure and how it integrates with OpenCode.
 * In practice, users would install the plugin via npm or copy it to .opencode/plugin/
 */

import type { Plugin } from "@opencode-ai/plugin";
import { CupcakePlugin } from "./src/index.js";

/**
 * Example 1: Use the Cupcake plugin directly
 *
 * Place this in .opencode/plugin/cupcake.ts
 */
export const MyCupcakePlugin: Plugin = CupcakePlugin;

/**
 * Example 2: Extend the Cupcake plugin with additional hooks
 *
 * This shows how you could add custom behavior alongside Cupcake policy enforcement
 */
export const ExtendedCupcakePlugin: Plugin = async (context) => {
  // Initialize Cupcake plugin
  const cupcakeHooks = await CupcakePlugin(context);

  return {
    // Include all Cupcake hooks
    ...cupcakeHooks,

    // Add additional custom hooks
    event: async ({ event }) => {
      if (event.type === "session.created") {
        console.log("New OpenCode session started!");
      }
    },
  };
};

/**
 * Example 3: Conditional Cupcake enforcement
 *
 * Only enforce policies in certain directories or for certain users
 */
export const ConditionalCupcakePlugin: Plugin = async (context) => {
  const { directory } = context;

  // Only enforce in production directories
  if (!directory.includes("/production/")) {
    console.log("[cupcake] Skipping policy enforcement outside production");
    return {}; // No hooks
  }

  // Enforce policies in production
  return await CupcakePlugin(context);
};
