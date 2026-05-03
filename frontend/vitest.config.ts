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
      // Default 5000 ms is tight for the mobile-smoke tests when CI VMs see
      // CPU contention (e.g. parallel Rust builds in the same workflow).
      // Happy paths run in <500 ms; this is purely headroom for cold imports
      // + jsdom environment setup under load.
      testTimeout: 15000,
      hookTimeout: 15000,
    },
  }),
)
