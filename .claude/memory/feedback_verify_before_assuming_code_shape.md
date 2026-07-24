---
name: Verify code shape before drafting plans / dispatching work
description: Read the actual existing code (traits, abstractions, types) before writing plans or dispatching implementation. Don't assume the codebase looks like the plan says it does.
type: feedback
originSessionId: 75332392-b9dc-4e06-b274-05f71e4842d6
---
When writing plans, briefing subagents, or making architectural recommendations, **read the code first**. Don't assume an abstraction doesn't exist just because the plan or spec doesn't mention it. Catching mismatches mid-execution costs the user tokens and time.

**Why:** During the monetisation refinement plan execution (2026-05-04), I drafted a plan to introduce an `AnimeDataSource` trait without first checking whether a similar abstraction existed. It did — `common::item::Repository` (`common/src/item.rs:57`) was already the data-source trait, and `server::mappers::anilist::Anilist` already implemented it converting AniList GraphQL → `common::Item`. The user's feedback: "you should record something to remind yourself to always look at the code before making this sort of assumption: it has a cost in tokens and time for me."

**How to apply:**
- Before writing a plan that introduces a new trait, struct, or abstraction, search the relevant crate / module for existing ones serving the same purpose. Use `mcp__serena__get_symbols_overview` or `Grep` for trait keywords (`trait`, `impl Trait for`).
- If a plan section assumes "the existing client returns shape X" or "we'll create trait Y," verify the assumption by reading at least the symbol overview of the relevant file before committing to the plan.
- When dispatching implementer subagents, do the verification step in the controller (you), not in the subagent — surfacing it via the subagent costs more.
- If you write a plan that *does* assume something unverified, flag the assumption explicitly in the plan ("assumes no existing data-source trait — verify before Phase 0") so it's caught at review.
- The cost calculus: 30 seconds of `get_symbols_overview` saves a 10-minute back-and-forth where the implementer or reviewer catches the mismatch.
