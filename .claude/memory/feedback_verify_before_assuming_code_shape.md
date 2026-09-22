---
name: Verify code shape before drafting plans / dispatching work
description: Read the actual existing code (traits, abstractions, types, runtime behaviour) before writing plans, dispatching implementation, or recommending infrastructure. Don't assume the codebase looks like the plan says it does.
type: feedback
originSessionId: 75332392-b9dc-4e06-b274-05f71e4842d6
modified: 2026-09-10T20:30:16.883Z
---
When writing plans, briefing subagents, or making architectural recommendations, **read the code first**. Don't assume an abstraction doesn't exist just because the plan or spec doesn't mention it. Catching mismatches mid-execution costs the user tokens and time.

**Why (instance 1 — abstractions, 2026-05-04):** During the monetisation refinement plan execution, I drafted a plan to introduce an `AnimeDataSource` trait without first checking whether a similar abstraction existed. It did — `common::item::Repository` (`common/src/item.rs:57`) was already the data-source trait, and `server::mappers::anilist::Anilist` already implemented it converting AniList GraphQL → `common::Item`. The user's feedback: "you should record something to remind yourself to always look at the code before making this sort of assumption: it has a cost in tokens and time for me."

**Why (instance 2 — infrastructure advice, 2026-09-10):** Asked where to deploy, I gave three reasons Cloud Run was a poor fit for this app *before* reading the relevant source. Two were wrong. (a) I claimed SSE would break — but `usePresence.ts:59` uses native `EventSource` and lines 64-69 explicitly document relying on browser auto-retry, so a platform request cap is just a transparent reconnect. (b) I claimed per-instance state broke multi-instance — but `services/calendar_events.rs:89` fans out over **Redis Pub/Sub**, so multi-instance was designed for from the start; the only per-instance state is a soft connection cap. Only the third (the in-process `tokio::time::interval` reconcile loop, `reconcile.rs:440`) was real — and the codebase had *already* anticipated it, since `ReconcileConfig::interval_secs = 0` exists and is documented as "used in tests and in deployments that don't want background work". The user had to push back ("anything we could change to make Cloud Run viable?") to get the verification done. **Platform recommendations are code claims.** This generalises the rule past plans: the same failure mode reaches hosting, cost and architecture advice, where it is harder to spot because nothing fails loudly.

**How to apply:**
- Before writing a plan that introduces a new trait, struct, or abstraction, search the relevant crate / module for existing ones serving the same purpose. Use `mcp__serena__get_symbols_overview` or `Grep` for trait keywords (`trait`, `impl Trait for`).
- If a plan section assumes "the existing client returns shape X" or "we'll create trait Y," verify the assumption by reading at least the symbol overview of the relevant file before committing to the plan.
- **Before saying a platform / deployment / runtime constraint applies, name the file and line that makes it true.** If you can't, it's UNKNOWN, not a constraint. State the evidence ("`usePresence.ts` opens an EventSource"), never the conclusion ("SSE won't work on Cloud Run").
- **Check whether the codebase already has the escape hatch.** A config knob that disables a feature (`interval_secs = 0`) is strong evidence someone already considered the deployment shape you are about to declare impossible.
- When dispatching implementer subagents, do the verification step in the controller (you), not in the subagent — surfacing it via the subagent costs more.
- If you write a plan that *does* assume something unverified, flag the assumption explicitly in the plan ("assumes no existing data-source trait — verify before Phase 0") so it's caught at review.
- The cost calculus: 30 seconds of `get_symbols_overview` saves a 10-minute back-and-forth where the implementer or reviewer catches the mismatch.

Related: [[user_gcp_cloud_run_experience]] — the learning-curve argument I leaned on in that bad recommendation was also wrong on the user's own facts. [[project_deploy_readiness_and_open_decisions]] records what the corrected analysis concluded.
