---
name: Parallel SDD tasks editing locale JSON cherry-pick conflict
description: When two parallel SDD subagents both add new top-level keys to the same i18n locale file, their commits cherry-pick fine individually but collide as JSON token conflicts when merging the second.
type: feedback
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
Locale JSON files (`frontend/src/locales/{en,pt}.json`) are editing hotspots — almost every UI feature touches them. When two SDD subagents work in parallel and both add keys to the same top-level namespace (e.g. `mobile.editor.*` and `mobile.schedule.*`), their isolated diffs both look fine, but cherry-picking the second on top of the first triggers a JSON conflict because the surrounding lines (closing braces, sibling keys) differ.

**Why:** Tracks 3 Tasks 11 and 12 hit this — Task 12 renamed `mobile.schedule.eyebrow` while Task 11 inserted a sibling `mobile.editor.*` block. Both diffs were locally clean; the conflict only surfaced at cherry-pick time on Windows, where it manifested as file-aliasing artifacts (the controller's working tree had unexpected files from the second agent's commit).

**How to apply:**
1. **Serialize JSON-touching work** in SDD plans — note which tasks edit `*.json` and either run them sequentially or batch them after parallel non-JSON work lands.
2. When parallel JSON-edit conflict is unavoidable, the controller resolves by stashing dirty files, cherry-picking, then dropping the stash — don't let the agent retry, the second one's conflict resolution will diverge from the first one's intent.
3. Worth flagging in implementer briefs: "If your task adds i18n keys to `en.json` / `pt.json`, expect to be the only agent doing so in this batch."
