---
name: Vitest test pollution from createWebHistory
description: Component tests must use createMemoryHistory; createWebHistory leaks state across files in the same vitest run
type: feedback
originSessionId: 7a8d378a-90cb-41a0-b043-addec4ba2a44
---
When mounting Vue Router in component tests, **always use `createMemoryHistory`**, never `createWebHistory`.

**Why:** `createWebHistory` shares the browser's history globally across the vitest worker. One test file's navigation pushes leak into the next file's router state, causing flaky redirect/auth tests when run as a full suite (but pass in isolation). Track 2 Phase A first surfaced this as a "loadFailed when GET /calendars/:id returns an error" flake in `errorScenarios.spec.ts`. Track 2 Phase B's `MyCalendarsPage.spec.ts` rewrite to `createMemoryHistory` resolved it without further changes — verified by 8 consecutive clean full-suite runs on 2026-05-01.

**How to apply:**
- In any new component test, prefer `import { createMemoryHistory } from 'vue-router'` and use it for the test router
- If you encounter a test that fails only in the full suite (not in isolation), check whether it or any sibling file uses `createWebHistory` and migrate it first
