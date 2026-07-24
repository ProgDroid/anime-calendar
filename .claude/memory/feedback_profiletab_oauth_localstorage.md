---
name: feedback_profiletab_oauth_localstorage
description: "ProfileTab reads localStorage name/avatar directly for OAuth users — those writes are load-bearing, not dead, despite the dead auth-store refs"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 2508ecb9-7396-43c8-b698-f64d520ddb72
---

The `localStorage.setItem('name'/'avatar')` writes in `stores/auth.ts` `oauthLogin` are **load-bearing, not dead**: `ProfileTab.vue` reads them directly via `localStorage.getItem('name'/'avatar')` to render an OAuth user's display name + avatar, because `/user/details` returns an empty `username` for OAuth users. Only the `auth.ts` store refs `name` / `user_avatar` were dead (written, exposed in the store object, read nowhere) — those were removed in Step 6 (M-15); the localStorage writes were kept.

**Why:** The audit's M-15 literal text said "remove the localStorage.setItem pair" — that would have blanked the OAuth profile. Following an audit finding verbatim without checking consumers introduces a regression.

**How to apply:** Before deleting "dead" localStorage writes, grep for direct `localStorage.getItem` consumers, not just store-ref reads. This bites specifically when F2-28 (frontend store-hygiene cluster) is picked up — it revisits `auth.ts` and the stores. Keep the OAuth name/avatar localStorage writes until ProfileTab sources them from the backend instead.
