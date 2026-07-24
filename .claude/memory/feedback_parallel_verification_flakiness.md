---
name: Run cargo + npm verification gates sequentially, not in parallel
description: Running `cargo test` / `cargo clippy` / `npm run test:unit` in parallel under heavy disk+CPU load produces spurious frontend test failures. Verify gates sequentially when validating release readiness.
type: feedback
originSessionId: 75332392-b9dc-4e06-b274-05f71e4842d6
---
When validating release-readiness across the Rust+Vue stack, **do not** run `cargo test --workspace`, `cargo clippy --workspace`, and `cd frontend && npm run test:unit` as parallel Bash tool calls in a single message. They share filesystem and CPU resources and produce phantom frontend test failures (likely `node_modules/.vite` cache races, jsdom timing under contention, or disk-write ordering).

**Why:** During Phase 1.5 verification on 2026-05-04, a parallel run reported `2 failed test files | 8 failed tests`. Re-running `npm run test:unit` sequentially immediately after — same code, same branch — produced `68 passed | 392 passed`. The cargo gates were clean both times. The frontend failures were not real.

**How to apply:**
- Run `cargo check`, `cargo test`, `cargo clippy` in parallel with each other if you must — they're all Rust, share `target/` cache cooperatively.
- Run `npm run test:unit` and `npm run lint` SEQUENTIALLY, not concurrent with cargo runs.
- If a frontend test "fails" but the assertion message looks suspicious (timeout, hydration race, "element not found" on stable DOM), re-run alone before debugging — odds are it's contention, not a real bug.
- Trust the sequential run as ground truth. The parallel run is a fast smoke check, not the verification gate.
