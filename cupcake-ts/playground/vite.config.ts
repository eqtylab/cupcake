import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@cupcake/policy': path.resolve(__dirname, '../policy/src/index.ts'),
    },
  },
  optimizeDeps: {
    include: ['react', 'react-dom', '@monaco-editor/react'],
  },
});
