---
name: Split backend + frontend into separate commits when changing a breaking API shape
description: When a single task changes the wire shape of an API the frontend already consumes, land backend and frontend as two separately-verified commits, not one mega-commit. User-validated preference on this codebase.
type: feedback
originSessionId: 76bbcb64-35b1-478f-a642-0b655f10cd65
---
**Rule:** When a task spans a server response shape change AND the frontend code that reads it, split into two commits — backend first, frontend second. Each gets its own ground-truth verification gate before commit.

**Why:** Three concrete benefits, validated by the user when given the option (b)+(x) on Task 0.11:

1. **Independent ground truth.** Backend gate is `cargo test --workspace + cargo clippy + redocly lint`; frontend gate is `npm run test:unit + lint + build`. Running them sequentially across two commits means each diff has a clean, single-stack verification stamp. A single mega-commit would either skip half the gates or make it impossible to localize a failure.
2. **Reviewable diffs.** Each commit is small enough to read end-to-end. Backend reviewers see SQL + Rust + OpenAPI; frontend reviewers see Vue + TS + locale + tests. No one has to context-switch mid-commit.
3. **Bisect-friendly.** If a bug surfaces later, git bisect lands on either the backend half (wire shape regression, mapper bug) or the frontend half (consumer bug, type mismatch) — not "the big shape change commit."

The intermediate state has a broken UI between the two commits, which is acceptable on this project because:

- Phase work lands on `main` directly.
- Nothing is deployed mid-phase (deploys happen at phase boundaries / verification gates).
- A frontend follow-up commit is queued before the backend commit even gets a green test light.

If those assumptions ever change (mid-phase deploys, branch-based feature work), reconsider — but on this project's flow, the split is the right default.

**How to apply:**

1. When triaging a task that touches both halves, ask the user "scope: full task one commit, or backend separately?" rather than picking silently.
2. Default recommendation when asked: split.
3. The backend commit message should call out the breaking shape change explicitly and note the frontend follow-up is queued. Example trailer used on `952c47e`: *"BREAKING for the frontend: response shape changed. Frontend follow-up must land before any deploy."*
4. The frontend commit lands as soon as both halves are green; don't sit on it.

**Counter-example (don't split):** Pure-additive backend changes that the existing frontend continues to ignore (new optional response field, new endpoint not yet consumed). Those are one commit.

**First seen:** Task 0.11 of co-editor-sharing. User asked "what impact does dropping pagination have?" and chose `(b) paginate owned only + (x) backend separately` from the offered scope options. Commits `952c47e` (backend) → `974b12a` (frontend) → `a635ccb` (verification gate).
