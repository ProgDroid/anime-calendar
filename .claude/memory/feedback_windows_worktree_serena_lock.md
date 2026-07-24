---
name: Windows worktree removal blocked by Serena LSP holding cwd
description: After activating a Serena project at a worktree path, `git worktree remove` fails with "process cannot access" — reactivate Serena to the source repo first, then retry
type: feedback
originSessionId: ae44063a-601c-45c4-b9ac-ac2c2751589f
---
On Windows, `git worktree remove .worktrees/<branch>` (and `Remove-Item -Recurse`) can fail with "The process cannot access the file ... because it is being used by another process" even after killing Vite/node dev servers running inside the worktree.

**Why:** The Serena MCP plugin's language server (and possibly its LSP indexer) sets the activated project's directory as its working directory. Even after the dev server is dead, Serena keeps a handle on the worktree path. Windows refuses recursive deletion while any process has cwd inside the tree.

**How to apply (cleanup ritual on Windows):**
1. Stop the dev server (`TaskStop` background bash, or kill via `Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like '*<worktree>*' }` → `Stop-Process`).
2. **Reactivate Serena to the source repo**: `mcp__plugin_serena_serena__activate_project` with the main project path (not the worktree).
3. Wait ~2 seconds for Serena to release the old cwd.
4. `git worktree remove` (or `Remove-Item -Recurse -Force`).
5. `git worktree prune` if the registry got out of sync from a partial earlier remove.

**Diagnostic command for finding the locker:**
```powershell
Get-CimInstance Win32_Process | Where-Object {
  $_.CommandLine -and $_.CommandLine -like '*<worktree-name>*'
} | Select-Object ProcessId, Name, CommandLine | Format-List
```

Tracking down a node process by `Path` is unreliable because all the swarm of nodes share the same `node.exe`; check `CommandLine` (or cwd) instead.
