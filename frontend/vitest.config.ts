import { fileURLToPath } from 'node:url'
import { mergeConfig, defineConfig, configDefaults } from 'vitest/config'
import viteConfig from './vite.config'

export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: 'jsdom',
      exclude: [...configDefaults.exclude, 'e2e/**'],
      root: fileURLToPath(new URL('./', import.meta.url)),
      setupFiles: ['./src/__tests__/setup.ts'],
      // Per-file module isolation in forked child processes. Without this, two
      // specs that import the same component (e.g. the duplicate Forgot/Reset
      // PasswordPage smoke + functional specs) can share a worker's module
      // cache: the component evaluates once under the first spec's
      // `vi.mock('axios')` and the second spec's re-mock never rebinds it,
      // producing an order-dependent "axios.post called 0 times" flake (F2-32).
      isolate: true,
      pool: 'forks',
      // Default 5000 ms is tight for the mobile-smoke tests when CI VMs see
      // CPU contention (e.g. parallel Rust builds in the same workflow).
      // Happy paths run in <500 ms; this is purely headroom for cold imports
      // + jsdom environment setup under load.
      testTimeout: 15000,
      hookTimeout: 15000,
    },
  }),
)
