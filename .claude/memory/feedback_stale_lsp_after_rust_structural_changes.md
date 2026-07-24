---
name: Stale rust-analyzer diagnostics after structural changes — trust cargo
description: Rust-analyzer's diagnostics shown via system-reminders go stale after renames, new struct definitions, removed functions. When LSP says "errors" but `cargo check --workspace` is clean, trust cargo.
type: feedback
originSessionId: 75332392-b9dc-4e06-b274-05f71e4842d6
---
After any Rust refactor that introduces / removes / renames a top-level item (trait, struct, function), the LSP diagnostics surfaced via `<new-diagnostics>` system-reminders may show **phantom errors** that don't reflect the actual filesystem state. The on-disk code can be perfectly correct while rust-analyzer is still serving stale results from before the change took effect.

**Why:** Observed three times in a row during the monetisation refinement Phase 0 work (2026-05-04):
1. After renaming `common::item::Repository` → `AnimeDataSource`: LSP claimed unresolved imports in `controllers/calendar.rs` etc. `cargo check` was clean.
2. After adding `CacheConfig` and `LimitsConfig` structs to `config/server.rs`: LSP claimed "cannot find type `CacheConfig`" with hints suggesting the wrong import. `cargo check` was clean.
3. After moving cache lookups into the new `CachedAnilist` adapter: LSP claimed `generate_item_key` / `Anilist` types weren't found and that `cached_anilist.rs` was "not in the module tree." `cargo check` was clean.

In each case the implementer's report had pasted clean `cargo build / test / clippy` output, and the LSP diagnostics arrived AFTER the dispatch — i.e., rust-analyzer indexed the post-change tree but couldn't reconcile its in-memory analysis cache.

**How to apply:**
- When `<new-diagnostics>` fires after a structural change, **don't** immediately re-dispatch a fix — first run `AWS_LC_SYS_PREBUILT_NASM=1 cargo check --workspace` (or `--workspace --tests` if test code is involved). If cargo is clean, the LSP is stale and the work is correct.
- Trust pasted compile output over LSP diagnostics. This is why the dispatch briefs ask implementers to paste the last 5 lines of `cargo build` raw, not just summarise.
- Save 1–2 minutes per false alarm by running `cargo check` BEFORE writing any "fix" reply.
- The LSP usually self-heals on the next file save in the IDE, so the user-visible diagnostics will catch up shortly.
