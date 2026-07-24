---
name: feedback_vitest_pool_forks_isolation
description: "vitest 4 default 'threads' pool shares a worker's module cache across files; use pool:'forks'+isolate to stop cross-file vi.mock leaks"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: ab83a158-ffc2-4fb3-a1dd-2857b860d545
---

The order-flaky `vi.mock('axios')` leak (F2-32) — symptom "axios.post called 0 times" / "mockResolvedValue is not a function", passes in isolation, ~30-50% per full run — was root-caused 2026-06-12 to vitest 4's **default `threads` pool sharing a worker's module cache across test files**. Two specs importing the same component (the duplicate `Forgot`/`ResetPasswordPage` smoke + functional specs in different dirs) let the component evaluate once under the first spec's axios mock; the second spec's re-mock never rebinds the already-cached component.

**Why:** default `isolate:true` resets per file, but `threads` pool reuse still leaks module identity in practice; forked child processes give true per-file isolation.

**How to apply:** set `pool: 'forks'` + explicit `isolate: true` in `vitest.config.ts` (done, with a comment). Also unify any duplicated `vi.mock('<module>')` factory *shapes* across specs so a residual leak degrades to a wrong-instance call, not a hard crash. Verified by 3× consecutive 485/485 full runs. Sibling of [[feedback_vue_router_mock_leaks_across_workers]] and [[feedback_test_command_no_pipe]] (run bare, capture to file — `npm run test:unit` is `vitest`, watch-mode in a TTY).
