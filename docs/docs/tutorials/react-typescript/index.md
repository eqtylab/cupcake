---
title: "React + TypeScript Tutorial"
description: "Writing policies for React applications with Cupcake"
---

# React + TypeScript Tutorial

This tutorial walks you through writing Cupcake policies for a React + TypeScript application. By the end, you'll have working policies that enforce your team's coding standards.

## Tutorial Scenario

In this tutorial, we'll solve a real-world problem: **enforcing the use of custom components**.

Your team has built a custom `DatePicker` component with consistent styling, validation, and behavior. However, Claude sometimes uses the basic HTML `<input type="date">` element instead, which causes issues:

- **Inconsistent styling** across different browsers
- **Design system violations** - doesn't match your UI library
- **Missing validation logic** - your custom component has built-in date range validation

We'll write a policy that blocks HTML date inputs and guides Claude to use your `DatePicker` component instead.

## What You'll Learn

1. **Setup** - Prerequisites and understanding hooks
2. **First Policy** - Writing a policy to enforce component usage
3. **First Signal** - Using signals to run validation scripts
4. **Obscure Rules** - Project-wide restrictions based on README content
