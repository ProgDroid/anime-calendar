# Audit Step 3 — Editor Composable + H-3 + F2-10 + F2-12 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close audit findings F2-11, F2-12, H-3, F2-10 by extracting one shared `useCalendarEditor` composable (folding in keyed drafts + display-name toasts), adding an advisory lock to `add_item`, and fixing the router-guard settings clobber.

**Architecture:** Two separately-verified commits. **Backend** first (additive `display` field on item SSE frames + `add_item` advisory lock) — old frontend ignores the new field. **Frontend** second (new composable both editor views consume, keyed drafts, display toasts, router-guard reconcile).

**Tech Stack:** Rust / Actix-Web / sqlx (backend); Vue 3.5 `<script setup>` + TypeScript + Pinia + vue-i18n + Vitest (frontend).

**Spec:** `docs/superpowers/specs/2026-06-11-step-3-editor-composable-design.md`

---

## File Structure

**Backend (commit 1):**
- Modify `server/src/services/calendar_events.rs` — add `display: String` to `ItemAdded`/`ItemRemoved`.
- Modify `server/src/controllers/calendar.rs` — `add_item` (advisory lock + display), `remove_item` (display), test harness (`build_item_app_services` + `item_app!`), new cap regression test.

**Frontend (commit 2):**
- Modify `frontend/src/types/sharing.ts` — `display` on the two item-event union members.
- Create `frontend/src/composables/useCalendarEditor.ts` — shared editor core.
- Modify `frontend/src/components/calendar/CalendarEditorViewDesktop.vue` — script only (template unchanged).
- Modify `frontend/src/components/calendar/CalendarEditorViewMobile.vue` — script only (template unchanged).
- Modify `frontend/src/router/index.ts` — guard reconcile.
- Modify `frontend/src/__tests__/routerGuard.spec.ts` — mirror new guard.
- Modify `frontend/src/components/calendar/__tests__/CalendarEditorViewDesktop.spec.ts` — draft + display tests.

---

# COMMIT 1 — Backend

### Task B1: Add `display` to item SSE frames

**Files:**
- Modify: `server/src/services/calendar_events.rs:21-32`

- [ ] **Step 1: Add the field to both item variants**

In `server/src/services/calendar_events.rs`, change the `ItemAdded` and `ItemRemoved` variants to:

```rust
    ItemAdded {
        media_id: i32,
        actor: String,
        display: String,
        v: i32,
        at: NaiveDateTime,
    },
    ItemRemoved {
        media_id: i32,
        actor: String,
        display: String,
        v: i32,
        at: NaiveDateTime,
    },
```

- [ ] **Step 2: Extend the `actor` doc comment**

Immediately above `#[derive(Debug, Clone, Serialize)]` on the enum, append to the existing `# actor field contract` doc block this paragraph:

```rust
/// `ItemAdded` / `ItemRemoved` additionally carry a `display` field: the
/// actor's human username (falling back to the numeric id string if the user
/// row can't be loaded). `display` is for rendering toasts; `actor` remains the
/// numeric id used by the self-echo filter. Do not conflate them.
```

- [ ] **Step 3: Verify it does not yet compile (publish sites lack `display`)**

Run: `AWS_LC_SYS_PREBUILT_NASM=1 cargo check -p server`
Expected: FAIL — `missing field `display` in initializer of `CalendarEvent::ItemAdded`` at `controllers/calendar.rs` (two sites). This is fixed in Task B2.

---

### Task B2: `add_item` advisory lock + display; `remove_item` display; harness wiring

**Files:**
- Modify: `server/src/controllers/calendar.rs` — `add_item` (1075-1150), `remove_item` (1152-1213), `build_item_app_services` (2197-2224), `item_app!` macro (2226-2242)

- [ ] **Step 1: Replace the `add_item` body**

Replace the entire `add_item` function (the `#[utoipa::path...]` block through the closing brace) with — note the two new params `pool` and `users`, the advisory-locked transaction, and the `display` lookup:

```rust
/// Add a single item to a calendar. Owner and active editors may call this.
/// The show-cap entitlement check applies only to the calendar owner
/// (owner-funded model: the owner's quota gates additions regardless of who
/// makes the request). The cap-check + insert run inside a per-owner
/// advisory-locked transaction so concurrent additions can't both pass the
/// cap and exceed it (H-3).
#[utoipa::path(
    post,
    path = "/calendars/{id}/items",
    operation_id = "add_item",
    tag = "calendars",
    params(("id" = i32, Path, description = "Calendar ID")),
    request_body = AddItemRequest,
    responses(
        (status = 200, description = "Item added or already present"),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
        (status = 402, description = "Show cap reached"),
        (status = 403, body = crate::controllers::auth::ErrorResponse),
        (status = 404, body = crate::controllers::auth::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[post("/calendars/{id}/items")]
#[allow(clippy::too_many_arguments)]
pub async fn add_item(
    path: web::Path<i32>,
    body: web::Json<AddItemRequest>,
    claims: Claims,
    authz: web::Data<SharingAuthz>,
    calendars: web::Data<CalendarMapper>,
    entitlement: web::Data<EntitlementService>,
    cache: web::Data<Cache>,
    publisher: web::Data<CalendarEventPublisher>,
    pool: web::Data<sqlx::PgPool>,
    users: web::Data<UserMapper>,
) -> HttpResponse {
    let calendar_id = path.into_inner();
    let actor_id = match claims.user_id() {
        Ok(id) => id,
        Err(e) => return e.error_response(),
    };
    let cal = match calendars.get_by_id_any_owner(calendar_id).await {
        Ok(c) => c,
        Err(e) => return e.error_response(),
    };
    if let Err(e) = authz.assert_can(actor_id, &cal, Action::ItemMutate).await {
        return e.error_response();
    }

    // Advisory-locked transaction: serialise the cap-check + insert against
    // concurrent additions counting toward the same owner's quota (H-3).
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return Error::Database(e).error_response(),
    };
    if let Err(e) = sqlx::query!("SELECT pg_advisory_xact_lock($1)", i64::from(cal.user_id))
        .execute(&mut *tx)
        .await
    {
        return Error::Database(e).error_response();
    }
    // Show-cap only applies to the owner (owner-funded model). The in-tx
    // variant observes the locked, pre-insert state authoritatively.
    if actor_id == cal.user_id
        && let Err(e) = entitlement
            .assert_can_add_show_in_tx(&mut tx, actor_id, body.item_id)
            .await
    {
        return e.error_response();
    }
    let affected =
        match CalendarMapper::add_item_idempotent_with(&mut tx, calendar_id, body.item_id).await {
            Ok(a) => a,
            Err(e) => return e.error_response(),
        };
    if let Err(e) = tx.commit().await {
        return Error::Database(e).error_response();
    }

    let _ = cache.invalidate_calendar(calendar_id).await;
    let _ = cache.invalidate_user_paged_calendars(cal.user_id).await;
    let _ = cache.invalidate_subscription(&cal.subscription_token).await;
    let display = users
        .get_user_by_id(actor_id)
        .await
        .ok()
        .map(|u| u.username)
        .unwrap_or_else(|| actor_id.to_string());
    let _ = publisher
        .publish_calendar(
            calendar_id,
            &CalendarEvent::ItemAdded {
                media_id: body.item_id,
                actor: actor_id.to_string(),
                display,
                v: cal.meta_version,
                at: chrono::Utc::now().naive_utc(),
            },
        )
        .await
        .map_err(|e| log::error!("publish ItemAdded: {e}"));
    HttpResponse::Ok().json(serde_json::json!({ "affected": affected }))
}
```

- [ ] **Step 2: Add `display` + `UserMapper` to `remove_item`**

In `remove_item`, add `users: web::Data<UserMapper>,` as the final parameter (after `publisher`). Then, inside the `Ok(affected) =>` arm, immediately before the `let _ = publisher` call, insert the lookup and add `display` to the frame:

```rust
            let display = users
                .get_user_by_id(actor_id)
                .await
                .ok()
                .map(|u| u.username)
                .unwrap_or_else(|| actor_id.to_string());
            let _ = publisher
                .publish_calendar(
                    calendar_id,
                    &CalendarEvent::ItemRemoved {
                        media_id: item_id,
                        actor: actor_id.to_string(),
                        display,
                        v: cal.meta_version,
                        at: chrono::Utc::now().naive_utc(),
                    },
                )
                .await
                .map_err(|e| log::error!("publish ItemRemoved: {e}"));
```

(The function already imports/uses `UserMapper` at the module level via `build_put_services`; no new `use` needed.)

- [ ] **Step 3: Wire `pool` + `UserMapper` into the item test harness**

Per `feedback_actix_web_data_extractor_ordering`, the new `web::Data` params must be registered on the test app or those handlers 500. Change `build_item_app_services` (around line 2197) to return two more `Data` values:

```rust
    async fn build_item_app_services(
        pool: sqlx::PgPool,
        l: &LimitsConfig,
    ) -> (
        web::Data<SharingAuthz>,
        web::Data<CalendarMapper>,
        web::Data<EntitlementService>,
        web::Data<Cache>,
        web::Data<CalendarEventPublisher>,
        web::Data<sqlx::PgPool>,
        web::Data<UserMapper>,
    ) {
        let entitlement = EntitlementService::new(
            SubscriptionMapper::from_pool(pool.clone()),
            ShowCountService::new(pool.clone()),
            l,
        );
        let sharing_authz = SharingAuthz::new(
            CalendarEditorMapper::from_pool(pool.clone()),
            entitlement.clone(),
        );
        let publisher = web::Data::new(CalendarEventPublisher::new(RedisPubSub::for_tests().await));
        (
            web::Data::new(sharing_authz),
            web::Data::new(CalendarMapper::from_pool(pool.clone())),
            web::Data::new(entitlement),
            web::Data::new(Cache::for_tests().await),
            publisher,
            web::Data::new(pool.clone()),
            web::Data::new(UserMapper::from_pool(pool)),
        )
    }
```

Then update the `item_app!` macro (around line 2226):

```rust
    macro_rules! item_app {
        ($pool:expr, $limits:expr) => {{
            let (sa, cm, ent, ch, pub_, pp, um) = build_item_app_services($pool, $limits).await;
            test::init_service(
                App::new()
                    .app_data(sa)
                    .app_data(cm)
                    .app_data(ent)
                    .app_data(ch)
                    .app_data(pub_)
                    .app_data(pp)
                    .app_data(um)
                    .app_data(jwt_data())
                    .service(add_item)
                    .service(remove_item),
            )
            .await
        }};
    }
```

- [ ] **Step 4: Compile**

Run: `AWS_LC_SYS_PREBUILT_NASM=1 cargo check -p server`
Expected: PASS (clean).

- [ ] **Step 5: Run existing item tests to confirm no regression**

Run: `AWS_LC_SYS_PREBUILT_NASM=1 cargo test -p server --lib editor_can_add_item non_member_cannot_add_item non_member_cannot_remove_item`
Expected: PASS — 3 tests. (Confirms the new `Data` wiring didn't turn these into 500s.)

---

### Task B3: Regression test — owner at show cap cannot add a new item

**Files:**
- Modify: `server/src/controllers/calendar.rs` (integration_tests module — add after `non_member_cannot_remove_item`, around line 2325)

This characterizes the preserved cap behavior through the new locked path.

- [ ] **Step 1: Add the test**

```rust
    #[tokio::test]
    async fn owner_at_show_cap_cannot_add_new_item_returns_402() {
        let pool = crate::test_helpers::test_pool().await;
        let owner = seed_user(&pool).await;
        let (cal_id, _) = seed_calendar(&pool, owner.id, "Capped Cal").await;
        // Fill the owner to the show cap (2 distinct shows).
        for item in [10_i32, 20_i32] {
            sqlx::query("INSERT INTO calendar_items (calendar_id, item_id) VALUES ($1, $2)")
                .bind(cal_id)
                .bind(item)
                .execute(&pool)
                .await
                .unwrap();
        }

        let app = item_app!(pool.clone(), &limits(5, 2));
        // New (untracked) item at cap → 402.
        let req = test::TestRequest::post()
            .uri(&format!("/calendars/{cal_id}/items"))
            .insert_header(("Cookie", format!("auth_token={}", owner.token)))
            .set_json(serde_json::json!({ "item_id": 99 }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::PAYMENT_REQUIRED,
            "owner at show cap adding a new item should get 402"
        );

        // Already-tracked item at cap → 200 (idempotent, not a new show).
        let req2 = test::TestRequest::post()
            .uri(&format!("/calendars/{cal_id}/items"))
            .insert_header(("Cookie", format!("auth_token={}", owner.token)))
            .set_json(serde_json::json!({ "item_id": 10 }))
            .to_request();
        let resp2 = test::call_service(&app, req2).await;
        assert_eq!(
            resp2.status(),
            StatusCode::OK,
            "re-adding an already-tracked item at cap should succeed"
        );

        cleanup_user(&pool, owner.id).await;
    }
```

- [ ] **Step 2: Run it**

Run: `AWS_LC_SYS_PREBUILT_NASM=1 cargo test -p server --lib owner_at_show_cap_cannot_add_new_item_returns_402`
Expected: PASS.

- [ ] **Step 3: Full backend suite + clippy**

Run: `AWS_LC_SYS_PREBUILT_NASM=1 cargo test -p server --lib`
Expected: PASS (count ≥ previous 358; +1 new test).

Run clippy on touched files (project pedantic+nursery allow-list — see `reference_clippy_command`):
`AWS_LC_SYS_PREBUILT_NASM=1 cargo clippy -p server --all-targets`
Expected: no new warnings.

- [ ] **Step 4: Commit backend**

```bash
git add server/src/services/calendar_events.rs server/src/controllers/calendar.rs
git commit -m "fix(sharing,calendar): item-frame display name + add_item advisory lock (F2-12, H-3)"
```

(No `.sqlx/` regen needed: `add_item_idempotent_with` and `pg_advisory_xact_lock` are already cached / parameter-only.)

---

# COMMIT 2 — Frontend

### Task F1: Add `display` to the item-event types

**Files:**
- Modify: `frontend/src/types/sharing.ts:44-45`

- [ ] **Step 1: Add the field**

Change the two item members of the `CalendarEvent` union to include `display: string`:

```ts
  | { type: 'item_added'; media_id: number; actor: string; display: string; v: number; at: string }
  | { type: 'item_removed'; media_id: number; actor: string; display: string; v: number; at: string }
```

- [ ] **Step 2: Type-check**

Run: `npm --prefix frontend run build`
Expected: this surfaces type errors in the two editor views (they still reference `frame.actor` in toasts — fine, `actor` still exists) — build should still PASS at this point because adding a field doesn't break existing reads. If it passes, proceed.

---

### Task F2: Create the `useCalendarEditor` composable

**Files:**
- Create: `frontend/src/composables/useCalendarEditor.ts`

- [ ] **Step 1: Write the file**

```ts
import { ref, computed, watch, onBeforeUnmount } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import axios from 'axios'
import api from '@/config/api'
import { toastService } from '@/services/toastService'
import { addItem, removeItem } from '@/services/calendars'
import type { Item } from '@/types/item'
import type { Calendar, EventStyle } from '@/types/calendar'
import type { Viewer, CalendarEvent } from '@/types/sharing'
import { useAuthStore } from '@/stores/auth'
import { useUserSettingsStore } from '@/stores/userSettingsStore'
import { useUsageStore } from '@/stores/usageStore'
import { useRecommendations } from '@/composables/useRecommendations'
import { usePresence } from '@/composables/usePresence'
import { getMySubscription } from '@/services/subscription'

type CalendarLanguage = 'english' | 'romaji' | 'native'

interface CalendarDraft {
  calendarName: string
  calendarLanguage: CalendarLanguage
  calendarEventStyle: EventStyle
  itemsInCalendar: Item[]
}

const DRAFT_DEBOUNCE_MS = 1000

/**
 * Shared core for both editor views (desktop + mobile). Owns editor state,
 * item mutations, submit, the SSE live-sync watcher, presence, recommendations,
 * the bootstrap (tier + calendar load + settings), and keyed draft persistence
 * (F2-11). Each view keeps only its own search-input glue + template.
 */
export function useCalendarEditor() {
  const { t } = useI18n()
  const route = useRoute()
  const router = useRouter()
  const authStore = useAuthStore()
  const userSettingsStore = useUserSettingsStore()
  const usage = useUsageStore()

  const { recommendations, calculateRecommendations } = useRecommendations()

  // ── Identity ──────────────────────────────────────────────────────────────
  const calendarId = Array.isArray(route.params.id) ? route.params.id[0] : route.params.id
  const isExistingCalendar = calendarId !== 'new' && calendarId !== undefined && calendarId !== ''
  const numericCalendarId = isExistingCalendar ? parseInt(calendarId as string, 10) : 0
  // F2-11: draft key is scoped to the route id so calendar A's draft can never
  // be restored under /calendar/B, and a desktop↔mobile viewport flip for the
  // same id reuses the same slot (no edit loss).
  const draftKey = `calendarDraft:${calendarId ?? 'new'}`

  const { viewers, lastEvent } = isExistingCalendar
    ? usePresence(numericCalendarId)
    : { viewers: ref<Viewer[]>([]), lastEvent: ref<CalendarEvent | null>(null) }

  // ── State ─────────────────────────────────────────────────────────────────
  const calendarName = ref('')
  const calendarLanguage = ref<CalendarLanguage>('english')
  const calendarEventStyle = ref<EventStyle>('timed')
  const itemsInCalendar = ref<Item[]>([])
  const submitLoading = ref(false)
  const calendarError = ref<string | null>(null)
  const currentCalendar = ref<Calendar | null>(null)
  const currentUserId = ref<number | null>(null)
  const originalItemIds = ref<Set<number>>(new Set())
  const isFreeTier = ref(true)
  const showCollisionBanner = ref(false)
  const localBaselineMetaVersion = ref(0)

  const isOwner = computed(() =>
    currentCalendar.value != null &&
    currentUserId.value != null &&
    currentCalendar.value.user_id === currentUserId.value,
  )
  const editorShowCount = computed(() =>
    isFreeTier.value && usage.loaded ? usage.showCount : null,
  )

  // ── Item mutations ──────────────────────────────────────────────────────────
  const addItems = (items: Item[]) => {
    const newItems = items.filter(
      item => !itemsInCalendar.value.some(c => c.id === item.id),
    )
    if (newItems.length === 0) return
    itemsInCalendar.value.push(...newItems)
    calculateRecommendations(itemsInCalendar.value)
    if (isFreeTier.value) void usage.refresh()
  }

  const removeItemFromCalendar = (id: number) => {
    itemsInCalendar.value = itemsInCalendar.value.filter(item => item.id !== id)
    calculateRecommendations(itemsInCalendar.value)
    if (isFreeTier.value) void usage.refresh()
  }

  const clearCalendar = () => {
    itemsInCalendar.value = []
    calculateRecommendations([])
  }

  // ── Draft persistence (F2-11) ───────────────────────────────────────────────
  function readDraft(): CalendarDraft | null {
    try {
      const raw = sessionStorage.getItem(draftKey)
      return raw ? (JSON.parse(raw) as CalendarDraft) : null
    } catch {
      return null
    }
  }

  function applyDraft(d: CalendarDraft) {
    if (d.calendarName !== undefined) calendarName.value = d.calendarName
    if (d.calendarLanguage !== undefined) calendarLanguage.value = d.calendarLanguage
    if (d.calendarEventStyle !== undefined) calendarEventStyle.value = d.calendarEventStyle
    if (d.itemsInCalendar !== undefined) {
      itemsInCalendar.value = d.itemsInCalendar
      calculateRecommendations(itemsInCalendar.value)
    }
  }

  function clearDraft() {
    try {
      sessionStorage.removeItem(draftKey)
    } catch {
      /* ignore */
    }
  }

  let draftTimer: ReturnType<typeof setTimeout> | null = null
  watch(
    [calendarName, calendarLanguage, calendarEventStyle, itemsInCalendar],
    () => {
      if (draftTimer !== null) clearTimeout(draftTimer)
      draftTimer = setTimeout(() => {
        try {
          sessionStorage.setItem(
            draftKey,
            JSON.stringify({
              calendarName: calendarName.value,
              calendarLanguage: calendarLanguage.value,
              calendarEventStyle: calendarEventStyle.value,
              itemsInCalendar: itemsInCalendar.value,
            }),
          )
        } catch {
          /* ignore */
        }
        draftTimer = null
      }, DRAFT_DEBOUNCE_MS)
    },
    { deep: true },
  )

  onBeforeUnmount(() => {
    if (draftTimer !== null) {
      clearTimeout(draftTimer)
      draftTimer = null
    }
    // Only the throwaway "new" draft is cleared on unmount; existing-calendar
    // drafts persist (keyed by id) so a reload / viewport flip keeps edits.
    if (!isExistingCalendar) clearDraft()
  })

  // ── Submit ──────────────────────────────────────────────────────────────────
  const finishSuccess = (name: string) => {
    toastService.success(t('calendar.updateSuccess', { name }))
    calendarName.value = ''
    itemsInCalendar.value = []
    clearDraft()
    router.push('/my-calendars')
  }

  const submitCalendar = async () => {
    const MAX_NAME_LENGTH = 100
    if (!calendarName.value) {
      calendarError.value = t('calendar.enterCalendarName')
      return
    }
    if (calendarName.value.length > MAX_NAME_LENGTH) {
      calendarError.value = t('calendar.nameMaxLength', { max_length: MAX_NAME_LENGTH })
      return
    }
    if (itemsInCalendar.value.length === 0) {
      calendarError.value = t('calendar.noItemsSelected')
      return
    }

    submitLoading.value = true
    calendarError.value = null
    try {
      if (currentCalendar.value) {
        const calId = currentCalendar.value.id

        // 1. Compute diff
        const currentIds = new Set(itemsInCalendar.value.map(i => i.id))
        const toAdd = [...currentIds].filter(id => !originalItemIds.value.has(id))
        const toRemove = [...originalItemIds.value].filter(id => !currentIds.has(id))

        // 2. Apply per-item changes (works for both owner and editor)
        if (toAdd.length > 0 || toRemove.length > 0) {
          try {
            await Promise.all([
              ...toAdd.map(id => addItem(calId, id)),
              ...toRemove.map(id => removeItem(calId, id)),
            ])
            originalItemIds.value = new Set(currentIds)
          } catch {
            calendarError.value = t('calendar.updateFailed')
            submitLoading.value = false
            return
          }
        }

        // 3. Save meta via PUT (owners only — editors get 403, handled silently)
        try {
          const calendar = {
            id: calId,
            name: calendarName.value,
            language: calendarLanguage.value,
            event_style: calendarEventStyle.value,
            items: itemsInCalendar.value,
          }
          const response = await api.put('/calendar', calendar)
          finishSuccess(response.data.name)
        } catch (err) {
          if (axios.isAxiosError(err) && err.response?.status === 403) {
            // Editor: meta save forbidden (expected). Item ops already succeeded.
            finishSuccess(calendarName.value)
          } else {
            calendarError.value = t('calendar.updateFailed')
          }
        }
      } else {
        const calendar: Omit<Calendar, 'id' | 'created_at' | 'updated_at'> = {
          name: calendarName.value,
          language: calendarLanguage.value,
          event_style: calendarEventStyle.value,
          items: itemsInCalendar.value,
        }
        const response = await api.put('/calendar', calendar)
        finishSuccess(response.data.name)
      }
    } catch {
      calendarError.value = t('calendar.updateFailed')
    } finally {
      submitLoading.value = false
    }
  }

  // ── Live sync (SSE) ─────────────────────────────────────────────────────────
  const reloadPage = () => {
    window.location.reload()
  }

  watch(lastEvent, frame => {
    if (!frame) return
    switch (frame.type) {
      case 'item_added':
        // Only media_id is available; no endpoint to fetch a full Item — skip
        // local list update. F2-12: toast the human display name, not the id.
        if (frame.actor !== String(authStore.userId)) {
          toastService.success(t('sharing.toasts.itemAdded', { actor: frame.display }))
        }
        break
      case 'item_removed':
        if (frame.actor !== String(authStore.userId)) {
          itemsInCalendar.value = itemsInCalendar.value.filter(i => i.id !== frame.media_id)
          toastService.success(t('sharing.toasts.itemRemoved', { actor: frame.display }))
        }
        break
      case 'meta_updated':
        if (frame.actor !== String(authStore.userId) && frame.v > localBaselineMetaVersion.value) {
          showCollisionBanner.value = true
        }
        break
      case 'member_joined':
        toastService.success(t('sharing.toasts.joined', { name: frame.display }))
        break
      case 'member_left':
        toastService.success(t(`sharing.toasts.left.${frame.reason}`))
        break
      case 'kick':
        toastService.error(t(`sharing.toasts.kick.${frame.reason}`))
        void router.push('/my-calendars')
        break
    }
  })

  // ── Bootstrap (tier + calendar load + settings + draft overlay) ──────────────
  void (async () => {
    try {
      const ent = await getMySubscription()
      isFreeTier.value = ent.tier !== 'paid'
    } catch {
      isFreeTier.value = true
    }
    if (isFreeTier.value) void usage.refresh()

    if (isExistingCalendar) {
      submitLoading.value = true
      try {
        const response = await api.get(`/calendars/${calendarId}`)
        const calendar: Calendar = response.data
        calendarName.value = calendar.name
        calendarLanguage.value = calendar.language
        calendarEventStyle.value = calendar.event_style ?? 'timed'
        itemsInCalendar.value = calendar.items
        currentCalendar.value = calendar
        originalItemIds.value = new Set(calendar.items.map(i => i.id))
        calculateRecommendations(itemsInCalendar.value)
        localBaselineMetaVersion.value = calendar.meta_version ?? 0
      } catch {
        calendarError.value = t('calendar.loadFailed')
      } finally {
        submitLoading.value = false
      }
      try {
        const settings = await userSettingsStore.fetchSettings()
        currentUserId.value = settings.user_id ?? null
      } catch {
        /* non-critical */
      }
    } else {
      const settings = await userSettingsStore.fetchSettings()
      currentUserId.value = settings.user_id ?? null
      calendarLanguage.value =
        settings.title_language_preference === 'Romaji'
          ? 'romaji'
          : settings.title_language_preference === 'Native'
            ? 'native'
            : 'english'
    }

    // F2-11: overlay a saved draft AFTER the server load so in-progress edits
    // win over the persisted server state on reload / viewport flip.
    const draft = readDraft()
    if (draft) applyDraft(draft)
  })()

  return {
    calendarId,
    isExistingCalendar,
    calendarName,
    calendarLanguage,
    calendarEventStyle,
    itemsInCalendar,
    submitLoading,
    calendarError,
    currentCalendar,
    isFreeTier,
    isOwner,
    editorShowCount,
    addItems,
    removeItemFromCalendar,
    clearCalendar,
    submitCalendar,
    viewers,
    showCollisionBanner,
    reloadPage,
    recommendations,
    calculateRecommendations,
  }
}
```

- [ ] **Step 2: Type-check**

Run: `npm --prefix frontend run build`
Expected: PASS (the composable is not yet imported anywhere; just confirm it compiles).

---

### Task F3: Refactor the Desktop view onto the composable

**Files:**
- Modify: `frontend/src/components/calendar/CalendarEditorViewDesktop.vue` (`<script setup>` block, lines 82-384 — template untouched)

- [ ] **Step 1: Replace the entire `<script setup lang="ts">…</script>` block**

```ts
<script setup lang="ts">
import { computed } from 'vue'
import type { Item } from '@/types/item'
import { useCalendarSearch } from '@/composables/useCalendarSearch'
import { useCalendarEditor } from '@/composables/useCalendarEditor'
import CalendarSettingsForm from '@/components/calendar/CalendarSettingsForm.vue'
import CalendarItemsList from '@/components/calendar/CalendarItemsList.vue'
import ItemSearchPanel from '@/components/calendar/ItemSearchPanel.vue'
import RecommendationsSection from '@/components/calendar/RecommendationsSection.vue'
import PresenceChip from '@/components/shared/PresenceChip.vue'

defineOptions({ name: 'CalendarEditorViewDesktop' })

const {
  isExistingCalendar,
  calendarName,
  calendarLanguage,
  calendarEventStyle,
  itemsInCalendar,
  submitLoading,
  calendarError,
  currentCalendar,
  isFreeTier,
  isOwner,
  editorShowCount,
  addItems,
  removeItemFromCalendar,
  clearCalendar,
  submitCalendar,
  viewers,
  showCollisionBanner,
  reloadPage,
  recommendations,
} = useCalendarEditor()

const {
  fetchedItems,
  selectedItems,
  loading: searchLoading,
  searchError,
  handleSearch,
  toggleItemSelection,
} = useCalendarSearch()

const loading = computed(() => searchLoading.value || submitLoading.value)

const addItemToCalendar = () => {
  const toAdd = fetchedItems.value.filter(item => selectedItems.value.includes(item.id))
  addItems(toAdd)
  selectedItems.value = []
}

const addItemToCalendarSingle = (item: Item) => addItems([item])
</script>
```

(The `<template>` and any `<style>` stay exactly as they are.)

- [ ] **Step 2: Type-check + existing desktop spec**

Run: `npm --prefix frontend run test:unit -- CalendarEditorViewDesktop`
Expected: PASS — the existing "collision banner" and "kick" tests still pass.

- [ ] **Step 3: Add the display-name toast test**

In `frontend/src/components/calendar/__tests__/CalendarEditorViewDesktop.spec.ts`, add inside the `describe`:

```ts
  it('item_added toast shows the actor display name, not the numeric id', async () => {
    const { wrapper } = await mountView()
    const { toastService } = await import('@/services/toastService')
    MockEventSource.lastInstance.emit(
      'message',
      JSON.stringify({
        type: 'item_added',
        media_id: 7,
        actor: '2',
        display: 'Alice',
        v: 1,
        at: '2026-01-01T00:00:00Z',
      }),
    )
    await flushPromises()
    expect(toastService.success).toHaveBeenCalledWith('Alice added an item')
    wrapper.unmount()
  })
```

- [ ] **Step 4: Add the keyed-draft tests**

Add this import at the top of the spec (with the other component imports):

```ts
import CalendarSettingsForm from '../CalendarSettingsForm.vue'
```

Add an `afterEach` sessionStorage reset (merge into the existing `afterEach`):

```ts
  afterEach(() => {
    vi.clearAllMocks()
    sessionStorage.clear()
  })
```

Then add:

```ts
  it('overlays a saved draft for the matching calendar id', async () => {
    sessionStorage.setItem(
      'calendarDraft:42',
      JSON.stringify({
        calendarName: 'Draft Name',
        calendarLanguage: 'english',
        calendarEventStyle: 'timed',
        itemsInCalendar: [],
      }),
    )
    const { wrapper } = await mountView('42')
    const form = wrapper.findComponent(CalendarSettingsForm)
    expect(form.props('name')).toBe('Draft Name')
    wrapper.unmount()
  })

  it('ignores a draft saved under a different calendar id', async () => {
    sessionStorage.setItem(
      'calendarDraft:99',
      JSON.stringify({
        calendarName: 'Other Draft',
        calendarLanguage: 'english',
        calendarEventStyle: 'timed',
        itemsInCalendar: [],
      }),
    )
    const { wrapper } = await mountView('42')
    const form = wrapper.findComponent(CalendarSettingsForm)
    // Server value ('Test Cal' from the api mock) — the 99-scoped draft is not applied.
    expect(form.props('name')).toBe('Test Cal')
    wrapper.unmount()
  })
```

- [ ] **Step 5: Run the desktop spec**

Run: `npm --prefix frontend run test:unit -- CalendarEditorViewDesktop`
Expected: PASS — 5 tests (2 original + 3 new).

---

### Task F4: Refactor the Mobile view onto the composable

**Files:**
- Modify: `frontend/src/components/calendar/CalendarEditorViewMobile.vue` (`<script setup>` block, lines 1-302 — template untouched)

- [ ] **Step 1: Replace the entire `<script setup lang="ts">…</script>` block**

```ts
<script setup lang="ts">
import { ref, computed, nextTick } from 'vue'
import { onBeforeRouteLeave, onBeforeRouteUpdate, RouterLink } from 'vue-router'
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import { useEditorSelectionStore } from '@/stores/editorSelection'
import { useCalendarEditor } from '@/composables/useCalendarEditor'
import UiSegmented from '@/components/ui/UiSegmented.vue'
import IconPlus from '@/components/ui/icons/IconPlus.vue'
import EditorItemsPanelMobile from './EditorItemsPanelMobile.vue'
import EditorSearchPanelMobile from './EditorSearchPanelMobile.vue'
import CalendarSettingsForm from './CalendarSettingsForm.vue'
import PresenceChip from '@/components/shared/PresenceChip.vue'

defineOptions({ name: 'CalendarEditorViewMobile' })

const { t } = useI18n()
const selection = useEditorSelectionStore()

const {
  isExistingCalendar,
  calendarName,
  calendarLanguage,
  calendarEventStyle,
  itemsInCalendar,
  submitLoading,
  calendarError,
  currentCalendar,
  isFreeTier,
  isOwner,
  editorShowCount,
  addItems,
  removeItemFromCalendar,
  clearCalendar,
  submitCalendar,
  viewers,
  showCollisionBanner,
  reloadPage,
  recommendations,
} = useCalendarEditor()

const loading = computed(() => submitLoading.value)
const itemCount = computed(() => itemsInCalendar.value.length)

// Tab state
type Tab = 'items' | 'search'
const tab = ref<Tab>('items')
const searchPanelRef = ref<InstanceType<typeof EditorSearchPanelMobile> | null>(null)

const tabOptions = computed(() => [
  { value: 'items', label: t('mobile.editor.itemsTab', { count: itemCount.value }) },
  { value: 'search', label: t('mobile.editor.searchTab') },
])

function jumpToSearch() {
  tab.value = 'search'
  void nextTick(() => {
    searchPanelRef.value?.focus()
  })
}

const addItemFromSearch = (items: Item[]) => {
  const before = itemsInCalendar.value.length
  addItems(items)
  selection.clear()
  // Switch back to the items tab only if something was actually added.
  if (itemsInCalendar.value.length > before) tab.value = 'items'
}

const addRecommendation = (item: Item) => addItems([item])

// Clear selection on route changes
onBeforeRouteUpdate(() => selection.clear())
onBeforeRouteLeave(() => selection.clear())
</script>
```

(The `<template>` stays exactly as it is. Note `currentCalendar`, `isOwner`, `isFreeTier`, `editorShowCount`, `showCollisionBanner`, `reloadPage`, `viewers`, `isExistingCalendar` are all consumed by the existing template bindings.)

- [ ] **Step 2: Type-check + existing mobile spec**

Run: `npm --prefix frontend run test:unit -- CalendarEditorViewMobile`
Expected: PASS — existing mobile tests still pass.

- [ ] **Step 3: Build (full type-check across both refactored views)**

Run: `npm --prefix frontend run build`
Expected: PASS.

---

### Task F5: Router-guard reconcile (F2-10)

**Files:**
- Modify: `frontend/src/router/index.ts:1-4, 135-160`
- Modify: `frontend/src/__tests__/routerGuard.spec.ts`

- [ ] **Step 1: Update the guard imports**

In `frontend/src/router/index.ts`, replace the `applySettings` import line (line 4) with:

```ts
import { useTheme } from '@/composables/useTheme'
import { i18n } from '@/plugins/i18n'
```

(Remove `import { applySettings } from '@/services/applySettings'` entirely.)

- [ ] **Step 2: Replace the settings block in `beforeEach`**

Replace lines 143-149 (the `// Fetch settings…` block) with:

```ts
  // Reconcile theme/locale from the server only for authenticated users.
  // fetchSettings() fabricates defaults for unauth users; applying those would
  // clobber localStorage (feedback_unauth_default_reconcile). Route theme/accent
  // through useTheme — the single source of truth — instead of mutating
  // data-theme directly, which would desync the theme toggle.
  if (!to.meta.public && authStore.isAuthenticated()) {
    const settings = await userSettingsStore.fetchSettings()
    useTheme().reconcileFromServer({
      theme_preference: settings.theme_preference,
      accent_preference: settings.accent_preference,
    })
    i18n.global.locale.value = settings.language_preference
  }
```

- [ ] **Step 3: Rewrite the router-guard spec to mirror the new guard**

Replace `frontend/src/__tests__/routerGuard.spec.ts` entirely with:

```ts
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createRouter, createMemoryHistory } from 'vue-router'
import { createPinia, setActivePinia } from 'pinia'
import { defineComponent } from 'vue'

// ── Mocks ──────────────────────────────────────────────────────────────────

const reconcileFromServer = vi.fn()
const localeRef = { value: 'en' }

vi.mock('@/composables/useTheme', () => ({
  useTheme: () => ({ reconcileFromServer }),
}))
vi.mock('@/plugins/i18n', () => ({
  i18n: { global: { locale: localeRef } },
}))
vi.mock('@/stores/auth', () => ({ useAuthStore: vi.fn() }))
vi.mock('@/stores/userSettingsStore', () => ({ useUserSettingsStore: vi.fn() }))

import { useAuthStore } from '@/stores/auth'
import { useUserSettingsStore } from '@/stores/userSettingsStore'

// ── Helpers ────────────────────────────────────────────────────────────────

const Dummy = defineComponent({ template: '<div />' })

/** Build a fresh in-memory router with the same beforeEach guard as the real one. */
function makeRouter() {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', redirect: '/my-calendars' },
      { path: '/login', name: 'Login', component: Dummy, meta: { public: true } },
      { path: '/my-calendars', name: 'MyCalendars', component: Dummy, meta: { requiresAuth: true } },
      { path: '/account/profile', name: 'account.profile', component: Dummy, meta: { requiresAuth: true } },
    ],
  })

  // Mirrors the guard in router/index.ts exactly
  router.beforeEach(async (to, _from, next) => {
    const authStore = useAuthStore()
    const userSettingsStore = useUserSettingsStore()

    await authStore.initAuth()

    if (!to.meta.public && authStore.isAuthenticated()) {
      const settings = await userSettingsStore.fetchSettings()
      reconcileFromServer({
        theme_preference: settings.theme_preference,
        accent_preference: settings.accent_preference,
      })
      localeRef.value = settings.language_preference
    }

    if (to.path === '/login' && authStore.isAuthenticated()) {
      next('/my-calendars')
    } else if (to.meta.requiresAuth && !authStore.isAuthenticated()) {
      next('/login')
    } else {
      next()
    }
  })

  return router
}

function mockAuth(authenticated: boolean) {
  vi.mocked(useAuthStore).mockReturnValue({
    isAuthenticated: () => authenticated,
    initAuth: vi.fn().mockResolvedValue(undefined),
  } as unknown as ReturnType<typeof useAuthStore>)
}

function mockSettings(result: object) {
  vi.mocked(useUserSettingsStore).mockReturnValue({
    fetchSettings: vi.fn().mockResolvedValue(result),
  } as unknown as ReturnType<typeof useUserSettingsStore>)
}

// ── Tests ──────────────────────────────────────────────────────────────────

describe('Router navigation guard', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    localeRef.value = 'en'
  })

  it('unauthenticated user navigating to a protected route is redirected to /login', async () => {
    mockAuth(false)
    mockSettings({ theme_preference: 'dark', accent_preference: 'coral', language_preference: 'en' })
    const router = makeRouter()
    await router.push('/my-calendars')
    expect(router.currentRoute.value.path).toBe('/login')
  })

  it('authenticated user can navigate to a protected route', async () => {
    mockAuth(true)
    mockSettings({ theme_preference: 'light', accent_preference: 'iris', language_preference: 'pt' })
    const router = makeRouter()
    await router.push('/my-calendars')
    expect(router.currentRoute.value.path).toBe('/my-calendars')
  })

  it('unauthenticated user can access public /login route', async () => {
    mockAuth(false)
    mockSettings({ theme_preference: 'dark', accent_preference: 'coral', language_preference: 'en' })
    const router = makeRouter()
    await router.push('/login')
    expect(router.currentRoute.value.path).toBe('/login')
  })

  it('reconciles theme + locale from server on authenticated protected navigation', async () => {
    mockAuth(true)
    mockSettings({ theme_preference: 'light', accent_preference: 'iris', language_preference: 'pt' })
    const router = makeRouter()
    await router.push('/my-calendars')
    expect(reconcileFromServer).toHaveBeenCalledWith({ theme_preference: 'light', accent_preference: 'iris' })
    expect(localeRef.value).toBe('pt')
  })

  it('does NOT reconcile for an unauthenticated user (no clobber of localStorage)', async () => {
    mockAuth(false)
    mockSettings({ theme_preference: 'dark', accent_preference: 'coral', language_preference: 'en' })
    const router = makeRouter()
    await router.push('/my-calendars')
    expect(reconcileFromServer).not.toHaveBeenCalled()
    expect(localeRef.value).toBe('en')
  })

  it('does NOT reconcile when navigating to a public route', async () => {
    mockAuth(true)
    const fetchSettings = vi.fn().mockResolvedValue({})
    vi.mocked(useUserSettingsStore).mockReturnValue({
      fetchSettings,
    } as unknown as ReturnType<typeof useUserSettingsStore>)
    const router = makeRouter()
    await router.push('/login')
    expect(fetchSettings).not.toHaveBeenCalled()
    expect(reconcileFromServer).not.toHaveBeenCalled()
  })
})
```

- [ ] **Step 4: Run the router-guard spec**

Run: `npm --prefix frontend run test:unit -- routerGuard`
Expected: PASS — 6 tests.

- [ ] **Step 5: Full frontend gates (sequential — `feedback_parallel_verification_flakiness`)**

Run each, in order, confirming PASS before the next:
1. `npm --prefix frontend run test:unit`  (expect ≥ prior 484 + new tests; all green)
2. `npm --prefix frontend run lint`  (clean)
3. `npm --prefix frontend run build`  (clean)
4. `npm --prefix frontend run test:e2e`  (39/39)

- [ ] **Step 6: Commit frontend**

```bash
git add frontend/src/types/sharing.ts frontend/src/composables/useCalendarEditor.ts \
  frontend/src/components/calendar/CalendarEditorViewDesktop.vue \
  frontend/src/components/calendar/CalendarEditorViewMobile.vue \
  frontend/src/components/calendar/__tests__/CalendarEditorViewDesktop.spec.ts \
  frontend/src/router/index.ts frontend/src/__tests__/routerGuard.spec.ts
git commit -m "refactor(calendar): shared useCalendarEditor + keyed drafts + display toasts + guard reconcile (F2-11, F2-12, F2-10)"
```

---

### Task B/F-final: Update AUDIT.md

**Files:**
- Modify: `G:\rustDev\anime-calendar\AUDIT.md` (append after the "Step 2 status" section)

- [ ] **Step 1: Append a Step 3 status section**

```markdown
## Step 3 status (2026-06-11 — fully shipped)

4 findings closed across 2 commits on `main`:

- **F2-12 + H-3** (backend): `ItemAdded`/`ItemRemoved` SSE frames gain a `display` field (actor's username, sourced via `UserMapper` like `MemberJoined`); item toasts no longer render raw numeric ids. `add_item` now runs its owner cap-check + insert inside a `pg_advisory_xact_lock(owner)` transaction (the `put` pattern), closing the concurrent-add cap bypass. New `owner_at_show_cap_cannot_add_new_item_returns_402` regression test; item test harness wires the new `pool` + `UserMapper` Data.
- **F2-11 + F2-12 + F2-10** (frontend): new `useCalendarEditor` composable owns the editor core shared verbatim between desktop + mobile (state, submit, SSE watcher, presence, bootstrap, draft persistence). Drafts are now keyed `calendarDraft:{id}` (no cross-calendar bleed; portable across viewport flips), persisted on both views, overlaid after server load. Item toasts use `frame.display`. Router guard mirrors `main.ts`: auth-guarded `useTheme().reconcileFromServer()` + locale instead of the unauth-clobbering `applySettings` path.

Verification (sequential): `cargo test -p server --lib` green (+1) · clippy clean in touched files · `npm run test:unit` green (+5) · lint + build clean · `npm run test:e2e` 39/39.

Remaining from the follow-up sequence: step 3's docket H-3-adjacent refactor is done; step 4 backlog (H-16/17/18, M-*, CR-1, L-*, F2 LOWs) remains.
```

- [ ] **Step 2: Commit**

```bash
git add AUDIT.md
git commit -m "docs(audit): record step 3 ship status (4 findings, 2 commits)"
```

---

## Self-Review Notes

- **Spec coverage:** F2-11 (Task F2 draft logic + F3 tests), F2-12 backend (B1/B2) + frontend (F1/F3), H-3 (B2/B3), F2-10 (F5), composable extraction (F2/F3/F4) — all mapped.
- **Type consistency:** `display: String` (Rust) ↔ `display: string` (TS); `addItems(items: Item[])` used identically in both views; `reconcileFromServer({theme_preference, accent_preference})` matches `useTheme`'s signature; `add_item_idempotent_with(&mut tx, …)` matches the existing `pub(crate)` helper at `mappers/calendar.rs:716`.
- **Behavior changes (intentional, flagged):** existing-calendar drafts now persist per-id and overlay server load (required to fix viewport-flip loss); desktop search-results no longer persisted; mobile `addItemFromSearch` switches tab only when an item was actually added.
- **Harness:** new `web::Data` params registered in `item_app!` per `feedback_actix_web_data_extractor_ordering`.
```
