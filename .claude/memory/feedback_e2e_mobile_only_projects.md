---
name: feedback-e2e-mobile-only-projects
description: All Playwright projects are mobile viewports — desktop-only testids never render in e2e; run e2e in every verification gate
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 02001aee-2b27-44bb-90dc-b0292787dc7a
---

All three Playwright e2e projects (mobile-chrome/safari/firefox) use mobile viewports, so `CalendarEditorViewMobile` renders in e2e — desktop-only testids like `editor-items-list` can never appear (mobile marker is `editor-tab-bar`).

**Why:** 6/39 e2e tests sat broken for a month because phase-end verification gates ran only `test:unit`; nothing exercised the specs. Also: stubbed SSE bodies deliver frames instantly on mount, so asserting intermediate editor UI races the redirect under test — assert the final contract (URL) only. Playwright glob `**/api/invitations/{token}` does NOT match the `/accept` sub-path; sub-path POSTs need their own stub or they fall through to the catch-all abort.

**How to apply:** Include `npm run test:e2e` in every phase-end verification gate alongside unit tests. When writing e2e specs against the editor, use mobile markers (or `.first()` over both variants' markers). Stub every sub-path the page actually calls.
