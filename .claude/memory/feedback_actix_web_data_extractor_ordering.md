---
name: actix-web Data<T> extractors fire BEFORE Claims auth check
description: When adding a new web::Data<T> param to a handler, every test app for that handler — including no-token / 401 tests — must register the data, otherwise the request 500s on missing app_data instead of returning the expected 401.
type: feedback
originSessionId: 381aef83-f290-413f-bbf3-53cf86e90b5a
---
When you add a new `web::Data<T>` parameter to an existing handler, you MUST register that data on every test app that exercises the handler — including tests that send no auth token and expect 401.

**Why:** actix-web extracts handler arguments left-to-right. `web::Data<T>` is extracted before `claims: Claims`, so a missing `app_data(web::Data::new(...))` registration produces a 500 (`AppData<T> not configured`) before the auth middleware ever rejects the request. The 401 test then asserts on the wrong status and either fails or — worse, if the assertion is loose — silently passes against the wrong code path.

**How to apply:**
- For each test that builds an `App::new()...service(handler)` inline, walk the handler's signature and register `web::Data` for every extractor.
- If a `build_X_services` factory exists, return the new `web::Data<T>` from it and update the corresponding `X_app!` macro to plumb it through.
- Tripping signal: a previously-green `..._without_token_returns_401` test starts returning 500. The fix is always "register the missing data," not "change the assertion."

Confirmed in commit `3ce5973` (Task 0.9 of co-editor sharing): adding `web::Data<SharingAuthz>` to `delete_calendar` required updating `delete_calendar_own_returns_200`, `_non_existent_returns_404`, AND `_without_token_returns_401`.
