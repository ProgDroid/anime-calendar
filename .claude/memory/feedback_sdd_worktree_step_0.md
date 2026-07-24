---
name: SDD isolation worktrees branch from stale HEADs — mandate Step 0 reset
description: Subagents dispatched into isolation worktrees can land on a stale main from when the worktree was created, producing work that diverges from current trunk. Every implementer brief must start with `git fetch && git reset --hard main` as Step 0.
type: feedback
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
When dispatching subagent implementers in `subagent-driven-development` (SDD) mode with `isolation: "worktree"`, the worktree is branched from whatever `main` was at worktree-creation time. If you've landed multiple commits since then in the controller session, the subagent will start work on a HEAD that's potentially many commits behind. They'll finish, commit, and report DONE, but the diff cherry-picks against current main produce conflicts or import errors against APIs that didn't exist in their base.

**Why:** Track 3 Tasks 9 and 13 both hit this. Task 9's worktree branched from `a0cc8bb` (~25 commits behind main `dc71a8a`); the agent used `useWindowSize` instead of `useViewportLayout` because the latter didn't exist in their base. Both required SendMessage re-dispatches with hard-reset instructions, wasting an iteration cycle each.

**How to apply:** Every implementer brief in an SDD flow MUST begin with this Step 0 (verbatim, before any task description):

```
## STEP 0 — MANDATORY: ensure your worktree is on top of `main`

```bash
git fetch origin || true              # may fail if origin is behind; that's fine
git log --oneline -1 main             # remember local main HEAD
git rev-parse HEAD                    # if NOT same as main HEAD, you are stale
git reset --hard main                 # idempotent if already on main
```

Reset to local `main`, NOT `origin/main` — origin may be behind local main.
```

**Also note:** the worktree directory is `<repo>/.claude/worktrees/<branch-name>/`. On Windows, `git worktree remove` can fail if Serena holds the cwd — reactivate Serena to the source repo first, then retry. (This second point is already in `feedback_windows_worktree_serena_lock.md`.)
