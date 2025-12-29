/// <reference types="vite/client" />

declare module '*?raw' {
  const content: string;
  export default content;
}

declare module '*.d.ts?raw' {
  const content: string;
  export default content;
}
