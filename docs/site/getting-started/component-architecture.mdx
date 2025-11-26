---
layout: "@/layouts/mdx-layout.astro"
heading: "Component Architecture"
description: "This section explains how the Equality UI library is structured under the hood — how components use Tailwind, CSS Modules, and the theme system to stay fully scoped, reusable, and conflict-free."
---

## Overview

Each UI component in the library is built to be:

- **Self-contained** – no global CSS imports or leaks
- **Themed** – all tokens resolve via the active `ThemeProvider`
- **Composable** – extendable through the `className` prop

Internally, every component follows this structure:

```
src/
  components/
    button/
      button.tsx
      button.module.css
```

## CSS Modules + Tailwind

Every component imports its own `.module.css` file. Inside that file, Tailwind utilities are applied using the `@apply` directive.

```css
@reference '../../theme/theme.module.css';

.button {
  @apply ring-offset-background rounded-md;
  @apply whitespace-nowrap text-sm font-medium;
  @apply transition-all duration-300;
  @apply inline-flex items-center justify-center gap-1;
  @apply focus-visible:ring-ring focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-offset-2;
  @apply disabled:cursor-not-allowed disabled:opacity-50;
  @apply [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0;
}
```

The `@tailwindcss/postcss` plugin processes these files during build, generating scoped class names automatically (e.g. `_button_container__abc123`).

And then it's used like this:

```tsx
import styles from "./button.module.css";

export function Button({ className, ...props }) {
  return <button className={cn(styles["button"], className)} {...props} />;
}
```

Because Tailwind runs at build time, there's no runtime overhead — just static CSS.

## Theme system

At the root level, the `<ThemeProvider>` injects all design tokens as CSS custom properties:

```css
--color-primary: hsl(210 40% 98%);
--color-foreground: hsl(240 4% 16%);
--text-xxs: 0.625rem;
```

Tailwind utilities in the components reference these tokens, so `bg-primary` maps directly to `--color-primary`.

When you change the theme via:

```tsx
<ThemeProvider theme="dark">
```

…the provider updates those variables, and every component automatically restyles itself — no rebuilds, no extra logic.

## Build process

The library uses Vite in library mode (or `tsup` if preferred) to compile everything cleanly:

1. **TypeScript** builds the component logic.
2. **PostCSS + Tailwind** transform `.module.css` files.
3. **CSS Modules** scope the output so class names never collide.
4. **Theme tokens** are embedded and exposed through the provider.
5. Output goes into `/dist` as both ESM and CJS modules.

This setup means host apps can import components without any Tailwind setup of their own.

## Layered isolation

| Layer                | Responsibility                                            |
| -------------------- | --------------------------------------------------------- |
| ThemeProvider        | Injects design tokens & controls light/dark modes         |
| Component CSS Module | Handles structure & layout with Tailwind utilities        |
| className Prop       | Allows host-level overrides per instance                  |
| Build Pipeline       | Compiles, scopes, and tree-shakes for minimal bundle size |

## Benefits

- Zero global CSS collisions
- Fully theme-aware via CSS variables
- Reusable across multiple projects
- Works out of the box in any Vite or React setup
- Easy to extend at the component or theme level
