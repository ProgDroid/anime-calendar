---
name: Feedback: Never pipe test commands through tail or other filters
description: Piping npm run test:unit through | tail -N causes Vitest to hang indefinitely; run test commands bare with no pipe
type: feedback
originSessionId: 0378eb37-d27b-4000-9314-fca2c4039899
---
Never run `npm run test:unit 2>&1 | tail -N` or any variant that pipes the test runner through another command.

**Why:** Piping prevents Vitest (and potentially other test runners) from detecting EOF / terminal close, causing it to hang indefinitely. The user ran the same bare command and it finished in ~30s; the piped version ran for 10+ minutes without completing.

**How to apply:** Run test commands bare — no `| tail`, no `| head`, no `| grep`. The Bash tool captures full output to a file automatically. If you only need the summary, read the output file after the run. Same caution applies to `cargo test` — though cargo exits reliably, the pipe is still unnecessary.

Correct form:
```
npm run test:unit
cargo test --workspace
```

Wrong form:
```
npm run test:unit 2>&1 | tail -50    ← hangs Vitest
cargo test --workspace 2>&1 | tail -40  ← unnecessary, risky
```
