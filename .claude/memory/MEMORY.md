# Memory Index - anime-calendar

- [📁 Historical milestones](MEMORY-history.md) — shipped tracks/phases (Design System, Tracks 2–4, Monetisation, Co-editor Phases 0–5, audit Phase 3). NOT auto-loaded; read on demand for past-work context.

## Project state & references

- [DEPLOY: readiness + open decisions](project_deploy_readiness_and_open_decisions.md) — read `docs/deployment-readiness.md` + the plan's Decisions-of-record table first; Tasks 1-8/15/18/19 shipped, Postgres + Redis vendors still open (2026-09-22).
- [User: GCP/Cloud Run experience](user_gcp_cloud_run_experience.md) — substantial prior Cloud Run work; don't price in learning curve.
- [Tech Stack](project_tech_stack.md) — Rust/Actix/PG/Redis backend; Vue3/TS/Tailwind/Pinia frontend (DaisyUI evicted).
- [Conventions](project_conventions.md) — error doc comments, DI via web::Data, tests in-file, all strings i18n (both locales).
- [Audit tracker](reference_deep_dive_plan.md) — findings + status live in `AUDIT.md` at repo root. Read first on any "continue the audit".
- [Next features backlog](reference_next_features.md) — `~/.claude/plans/anime-calendar-next-features.md`; incl. cache endpoint lockdown for admin dashboard.
- [Tooling suggestions](project_tooling_suggestions.md) — skills/hooks/commands/agents to build next.
- [Design redesign tracks](reference_design_redesign_tracks.md) — specs/plans under `docs/superpowers/`; sequencing 1→2→4→3 (all shipped).
- [Track 2 follow-up plans](reference_track_2_followup_plans.md) — design-diff polish + a11y items; airing-count done.
- [402 reason routing — add a cap gate](reference_402_reason_routing.md) — Error::PaymentRequired{reason} + axios interceptor + UpgradeInterruptModal + i18n key.
- [Project clippy command](reference_clippy_command.md) — pedantic+nursery with curated allow-list, not bare `-D warnings`. In every implementer brief.
- [Audit trails — backlog](reference_audit_trails_backlog.md) — unified audit-trail subsystem deferred to a future spec.
- [2FA gate for invitations — backlog](reference_2fa_invitation_gate_backlog.md) — layer 2FA onto invite send+accept once 2FA ships.
- [Data-source: AniList stays (2026-05-05)](project_data_source_spike_outcome.md) — MAL no episode airing; AnimeSchedule/Kitsu block commercial; AniList sole source. Spec 4 still viable.
- [Dev DB migration drift](project_dev_db_migration_drift.md) — recurred; `sqlx migrate run` blocked again (2026-06-12). psycopg2 bypass + SHA-384 _sqlx_migrations bookkeeping (recipe in file).
- [Backend tests need PG + Redis](project_backend_test_infra_redis.md) — copy-paste throwaway-container recipe (no migration drift); lone subscribe_feed panic = Redis down, not a regression.
- [Design tokens text variants](project_design_tokens_text_variants.md) — `--accent-1-text`/`--danger-text` for AA text on neutral bg; bare tokens for surfaces only.

## Backend / Rust / sqlx

- [sqlx offline cache](feedback_sqlx_offline_cache.md) — regen + commit `.sqlx/` after any query-macro change; psycopg2 bypass for checksum drift.
- [JwtSecret expose_secret](feedback_jwt_secret_pattern.md) — concrete expose_secret() method; `use secrecy::ExposeSecret` not needed (warns unused).
- [Error::error_response leaks Display](feedback_error_response_redaction.md) — any variant interpolating a detail leaks via to_string() fallback; redact centrally.
- [Error variant for authz](feedback_error_variant_for_authz_failure.md) — 403 wrong-role; 404 anti-enum; 401 not-logged-in; 402 tier gate.
- [utoipa v5 schema](feedback_utoipa_schema_patterns.md) — value_type field-level only; query params need IntoParams; description invalid at field level.
- [utoipa rename ≠ serde rename](feedback_utoipa_serde_rename.md) — pair #[param(rename)] with #[serde(rename)] or query deser diverges from docs.
- [utoipa operationId collisions](feedback_utoipa_operation_id_collisions.md) — derived from bare fn name; same-named handlers collide silently; set operation_id explicitly.
- [Soft delete patterns](feedback_soft_delete_patterns.md) — partial unique indexes, cascade-in-tx, audit-trail preservation.
- [sqlx TIMESTAMP vs TIMESTAMPTZ](feedback_sqlx_timestamp_types.md) — TIMESTAMPTZ→DateTime<Utc>; TIMESTAMP→NaiveDateTime (match existing user cols).
- [NaiveDateTime→TIMESTAMP truncates ns](feedback_naivedatetime_postgres_microsecond_truncation.md) — round-trip equality flakes on µs; build deadlines from date literals.
- [NaiveDateTime UTC sentinel for dates](feedback_naivedt_utc_sentinel_for_dates.md) — append 'Z' before new Date() to anchor UTC, else per-tz date drift.
- [sqlx same-param SET+comparison poison](feedback_sqlx_param_assignment_vs_comparison.md) — same $N in SET and comparison fails PREPARE; use distinct params + .clone().
- [New migration FK widths match parent PK](feedback_migration_fk_width_must_match_parent_pk.md) — BIGINT REFERENCES integer_pk breaks sqlx at compile; mirror SERIAL→INTEGER.
- [Default new-mapper shape](feedback_new_mapper_default_shape.md) — every method wraps an `_in_tx` static; `_with` extinct; pool callers get `*_from_pool`; tests test_tx().
- [Mapper _in_tx static helper](feedback_in_tx_static_helper_pattern.md) — cross-mapper atomic writes expose `_in_tx(conn,...)` statics; caller owns the tx.
- [Audit lock arms use locked conn](feedback_audit_lock_in_tx_consistency.md) — under pg_advisory_xact_lock every op must use `_in_tx` on &mut *tx, not pool-bound.
- [Don't use #[sqlx::test]](feedback_no_sqlx_test_use_test_pool.md) — use test_pool() (shared + cleanup) or test_tx() (rollback). Brief MUST say so.
- [sqlx multi-mapper pool clone](feedback_sqlx_test_multi_mapper.md) — pool.clone() for all-but-last mapper; PgPool clone is cheap.
- [Cache invalidation completeness](feedback_cache_invalidation.md) — invalidate all keys of a mutated resource (primary + export + subscription) together.
- [Item lacks MediaStatus + cache versioning](feedback_anilist_item_no_status.md) — adding Item fields invalidates Redis blobs; bump cache prefix. airing_schedule usually suffices.
- [Derive over extend cached blobs](feedback_derive_over_extend_cached_blobs.md) — check if existing fields encode the answer before adding to a cached entity.
- [Pagination = Anilist fan-out ceiling](feedback_pagination_as_anilist_fanout_ceiling.md) — /calendars batches Anilist per page; pagination bounds worst-case. Don't drop without a replacement.
- [icalendar VALARM TRIGGER in seconds](feedback_icalendar_duration_seconds_format.md) — Duration::minutes(30)→`-PT1800S` not `-PT30M`; assertions must match seconds.
- [SHA-256 for high-entropy tokens](feedback_token_hashing_high_entropy.md) — 256-bit OS-RNG tokens use SHA-256 (matches reset/verify); argon2id only for low-entropy.
- [Anti-enumeration always-200](feedback_anti_enumeration_always_200.md) — resend/forgot return 200 on ALL paths incl internal errors; `let ok=||...` closure.
- [Reconcile conditional UPDATE guard](feedback_reconcile_conditional_update.md) — WHERE id=$1 AND current_period_end<=$end lets slow safety-net coexist with fast webhook; 0 rows = safe.
- [stripe_webhook String error chain](feedback_stripe_webhook_string_error_chain.md) — webhook handlers return Result<_,String>; bridge ServerResult with .map_err(|e|e.to_string())?.
- [async-stripe rc.5 module layout](feedback_async_stripe_rc5_module_layout.md) — type-to-crate map (Expandable in stripe_types, period on SubscriptionItem) + feature flags.
- [Stripe webhook user-id resolution](feedback_stripe_webhook_user_mapping.md) — stamp subscription_data.metadata.user_id at Checkout; carries through all events.
- [Billing endpoints take no caller id](feedback_billing_endpoint_no_request_body.md) — self-service billing resolves target from claims, never request body.
- [Sharing helpers in services/sharing.rs](feedback_sharing_helpers_in_services.md) — suspend/restore called by webhook + reconcile must live in services/, not controllers/.
- [No global auth middleware](feedback_no_global_auth_middleware.md) — Claims decoded per-handler via FromRequest; per-user rate limits in the service layer.
- [actix-governor path exemption](feedback_actix_governor_path_exempt.md) — (superseded by in-house rate_limit.rs) custom KeyExtractor path→sentinel-IP whitelist.
- [Data<T> extractors fire before Claims](feedback_actix_web_data_extractor_ordering.md) — register new web::Data<T> params on every test app incl 401 tests or they 500.
- [actix-web 4 service futures NOT Send](feedback_actix_web_4_service_futures_not_send.md) — middleware futures must be LocalBoxFuture not BoxFuture+Send; push back on reviewers.
- [tokio broadcast Lagged break on kick](feedback_tokio_broadcast_lagged_kick.md) — Lagged on a must-deliver (kick/evict) channel = missed signal; break defensively.
- [Redis NUMSUB fleet subscriber gate](feedback_redis_numsub_fleet_subscriber_gate.md) — PUBSUB NUMSUB for global "anyone watching?", not local Sender::receiver_count (one replica only).
- [Singleton loop advisory lock placement](feedback_singleton_loop_advisory_lock_outside_tested_fn.md) — pg_try_advisory_lock in a wrapper OUTSIDE run_pass; inside it, parallel tests contend on the global key.
- [RAII guards into async_stream::stream!](feedback_stream_lifetime_raii_guard.md) — `let _g=guard;` as first line inside the block or it drops before first poll.
- [Stale rust-analyzer after structural changes](feedback_stale_lsp_after_rust_structural_changes.md) — run `cargo check` before trusting <new-diagnostics> after renames/new structs.

## Frontend / Vue / testing

- [Axios 401 refresh interceptor](feedback_axios_refresh_interceptor.md) — `_retried` guard; window.location.href not router.push; public-route prefix skips redirect.
- [Runtime public config](feedback_vite_docker_env_vars.md) — `/api/public-config` at runtime, not `VITE_*`. Image env-agnostic.
- [Extending /public-config](feedback_public_config_extension.md) — update loadPublicConfig map + e2e stub (bootstrap hard-fails) + const-from-config fallback; display must share source with enforcement.
- [httpOnly cookie migration](feedback_httponly_cookie_migration.md) — 5-part contract: nginx strips /api, Vite mirror, axios withCredentials, CORS creds, cookie flags.
- [ProfileTab OAuth localStorage load-bearing](feedback_profiletab_oauth_localstorage.md) — localStorage name/avatar feed ProfileTab OAuth display; don't delete (bites F2-28).
- [Unauth defaults aren't server truth](feedback_unauth_default_reconcile.md) — fetchers fabricating unauth defaults clobber localStorage in server-wins reconcile; guard on auth.
- [SSE events echo to actor](feedback_sse_event_actor_filter.md) — events reach the originating subscriber; guard frame.actor (now actor_id) or user sees own actions.
- [Possession tokens via sessionStorage stash](feedback_high_entropy_token_handoff_via_session_storage.md) — not ?redirect= query; use inviteRedirect.ts (stash + consume-once).
- [Tier-gated cosmetic apply](feedback_tier_gated_apply_pattern.md) — preserve stored pref; resolve to default at apply-time via resolveValue(stored,isPaid).
- [i18n `<i18n-t>` named slots](feedback_i18n_t_named_slots.md) — splice Vue fragments via {slot} + <template #slot>; avoid HTML-in-JSON / v-html.
- [Confirm-dialog async ordering](feedback_confirm_dialog_async_pattern.md) — close modal first → guard → clear id in finally.
- [ref-on-component focus trap](feedback_vue_ref_on_component_focus.md) — template ref on a component is a proxy; .focus() no-ops unless defineExpose'd. Test activeElement.
- [Tailwind v4 @theme gaps](feedback_tailwind_v4_theme_gaps.md) — not every token is a utility; .font-display overrides text-* sizes; icons hardcode 24×24.
- [Tailwind v4 @theme var() chain](feedback_tailwind_v4_token_runtime_chain.md) — per-[data-theme] overrides propagate at runtime; HMR for token edits flaky, hard-reload.
- [axe-core oklch false positives](feedback_axe_oklch_false_positives.md) — don't trust axe contrast on OKLCH; re-verify canvas + alpha-composite + transitions-off.
- [Vue testing patterns](feedback_vue_testing_patterns.md) — data-testid; createMemoryHistory; `findAll()[0]!`; Pinia mock cast; defineOptions({name}).
- [Vitest createWebHistory pollution](feedback_test_history_pollution.md) — use createMemoryHistory; createWebHistory leaks router state across specs.
- [vi.mock('vue-router') leaks across workers](feedback_vue_router_mock_leaks_across_workers.md) — use createMemoryHistory + router.push override; never vi.mock vue-router.
- [vitest pool forks isolation](feedback_vitest_pool_forks_isolation.md) — vitest 4 default `threads` shares module cache across files; `pool:'forks'`+isolate fixes cross-file vi.mock leaks (F2-32).
- [UiModal Teleport breaks wrapper.find()](feedback_uimodal_teleport_tests.md) — mount attachTo: document.body, query document.body.
- [UiModal props must be camelCase](feedback_uimodal_prop_camel_case.md) — `:ariaLabel` not `:aria-label`; vue-tsc strict doesn't honor kebab→camel on component props.
- [jsdom + Teleport offsetParent](feedback_jsdom_focusable_offsetparent.md) — don't filter focusable by offsetParent!==null in Teleport components; jsdom returns null.
- [Vue 3.5 KeepAlive + jsdom nav crash](feedback_vue_keepalive_jsdom_navigation.md) — navigating away from a KeepAlive route in Vitest crashes Vue; spy router.push instead.
- [AccountPage tab couples to spec router stub](feedback_account_tab_router_stub_coupling.md) — new tab needs AccountPage.spec router stub update or all 11 tests throw.
- [Smoke test timer leaks](feedback_smoke_test_timer_leak.md) — onMounted setTimeout(router.push) leaks real timers; reject the API or fake timers.
- [npm run build is a separate gate](feedback_typecheck_gate_separate_from_unit.md) — vitest doesn't run vue-tsc; run build AND lint before DONE.
- [Never pipe test commands through tail](feedback_test_command_no_pipe.md) — `npm run test:unit | tail` hangs Vitest; run bare (Bash tool captures to file).
- [Playwright body locator fragility](feedback_playwright_body_locator_fragility.md) — expect(body).toBeVisible() fails (zero area); use a post-mount marker.
- [Playwright page.route REVERSE order](feedback_playwright_route_registration_order.md) — last-registered wins; catch-all `**/api/**` abort FIRST, specific stubs LAST.
- [Playwright SSE stub + GET/POST disambiguation](feedback_playwright_sse_stub.md) — fulfill with `text/event-stream` + `\n\n` frame; branch one route handler on `request().method()` (don't double-register).
- [e2e projects are mobile-only](feedback_e2e_mobile_only_projects.md) — desktop testids never render in e2e; run test:e2e every gate; stub sub-paths explicitly.

## Process / workflow

- [Verify code shape before assuming](feedback_verify_before_assuming_code_shape.md) — read existing traits/abstractions before plans/subagents; 30s overview beats mid-flight fix.
- [Verify type shapes in briefs](feedback_implementer_brief_must_verify_types.md) — every type/method/derive in a code stub must be verified vs source first.
- [Grep each layer before plans](feedback_grep_codebase_layers_before_drafting_plan.md) — plans from spec alone get wrong ID types/fields/layout; 5 min grep saves 30.
- [Verify config schema before plans](feedback_grep_config_shape_before_planning.md) — grep the Config struct before specifying TOML; "no code changes" is the tell.
- [Split BE+FE on breaking API shape](feedback_split_backend_frontend_on_breaking_api_shape.md) — ship backend + frontend as two separately-verified commits, not one mega-commit.
- [Run verification gates sequentially](feedback_parallel_verification_flakiness.md) — parallel cargo + npm tests give spurious FE failures; sequential is ground truth.
- [SDD worktree Step 0 reset](feedback_sdd_worktree_step_0.md) — implementer briefs MUST start `git fetch && git reset --hard main` (local main).
- [Parallel SDD locale JSON conflicts](feedback_sdd_parallel_json_conflicts.md) — parallel agents editing the same locale namespace cherry-pick-conflict; serialize JSON tasks.
- [Windows worktree + Serena lock](feedback_windows_worktree_serena_lock.md) — `git worktree remove` fails when Serena holds cwd; reactivate to source repo first.
