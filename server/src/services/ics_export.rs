use chrono::{DateTime, Duration};
use common::calendar::Calendar as CommonCalendar;
use common::id::Id;
use common::item::AnimeDataSource as _;
use common::language::Language;
use icalendar::{Alarm, Calendar as Ics, Component as _, Event, EventLike as _};
use log::debug;
use sqlx::PgPool;

use crate::entity::calendar::{Calendar as CalendarEntity, Language as LanguageEntity};
use crate::entity::subscription::Tier;
use crate::error::Error;
use crate::mappers::user_settings::UserSettingsMapper;
use crate::services::cached_anilist::CachedAnilist;
use crate::services::entitlement::EntitlementService;
use crate::ServerResult;

/// Defense-in-depth cap on per-event VALARMs. Settings validation already
/// rejects writes with more than 5 entries, but the renderer enforces the
/// same bound in case older rows slipped through before the validation
/// landed.
const MAX_VALARMS_PER_EVENT: usize = 5;

/// Renders a calendar to a complete .ics document string.
///
/// Phase 2: tier-aware per-user VALARM emission and event-style branching.
/// - Free users always get exactly one 30-min VALARM (server-side anti-bypass
///   — never trusts the stored offsets array).
/// - Pro users get one VALARM per stored offset (deduped, sorted, capped at
///   `MAX_VALARMS_PER_EVENT`).
/// - `event_style == "timed"` emits `DTSTART:datetime` events with a 24-min
///   duration; `"all_day"` emits `DTSTART;VALUE=DATE`.
#[derive(Clone)]
pub struct IcsExportService {
    pool: PgPool,
    cached_anilist: CachedAnilist,
    user_settings_mapper: UserSettingsMapper,
    entitlement: EntitlementService,
}

impl IcsExportService {
    #[must_use]
    pub const fn new(
        pool: PgPool,
        cached_anilist: CachedAnilist,
        user_settings_mapper: UserSettingsMapper,
        entitlement: EntitlementService,
    ) -> Self {
        Self {
            pool,
            cached_anilist,
            user_settings_mapper,
            entitlement,
        }
    }

    /// Render the calendar identified by `calendar_id` as a full .ics document.
    /// Looks up the calendar by id without an owner check — callers are
    /// responsible for authorisation. Empty calendars (no items, or items
    /// with no resolvable `AniList` data) produce a minimal but valid .ics.
    ///
    /// # Errors
    /// - `Error::NotFound` if the calendar id doesn't exist or is soft-deleted
    /// - sqlx errors for DB failure
    pub async fn render(&self, calendar_id: i32) -> ServerResult<String> {
        let entity = self.load_calendar_by_id(calendar_id).await?;
        let item_ids: Vec<Id> = entity
            .item_ids
            .iter()
            .filter_map(|id| Id::new(i64::from(*id)))
            .collect();

        // Resolve VALARM offsets from owner tier + (Pro only) stored settings.
        // Free users are NEVER given the chance to bypass the 30-min default —
        // we hardcode here, not after fetching settings. This is the renderer's
        // load-bearing security invariant.
        let valarm_offsets = self.resolve_valarm_offsets(entity.user_id).await?;
        let event_style = entity.event_style.clone();

        if item_ids.is_empty() {
            return Ok(empty_calendar_ics(&entity.name));
        }

        let items = self.cached_anilist.get_items(item_ids).await;

        if items.is_empty() {
            return Ok(empty_calendar_ics(&entity.name));
        }

        let common_cal = CommonCalendar {
            id: Id::new(i64::from(entity.id)).ok_or(Error::NotFound)?,
            items,
            language: entity.language.to_common_language(),
            name: entity.name.clone(),
            created_at: entity.created_at,
            updated_at: entity.updated_at,
        };

        Ok(format!(
            "{}",
            render_common_calendar(&common_cal, &event_style, &valarm_offsets)
        ))
    }

    /// Resolve the per-event VALARM offsets for a given calendar owner.
    ///
    /// Free → `[30]`, hardcoded; settings ignored entirely.
    /// Paid → load `reminder_offsets_minutes`, dedup, sort, truncate to
    /// `MAX_VALARMS_PER_EVENT`. Empty array means no VALARMs (Pro user's
    /// deliberate choice).
    async fn resolve_valarm_offsets(&self, user_id: i32) -> ServerResult<Vec<i32>> {
        let tier = self.entitlement.effective_tier(user_id).await?;
        match tier {
            Tier::Free => Ok(vec![30]),
            Tier::Paid => {
                let settings = self.user_settings_mapper.get_user_settings(user_id).await?;
                let mut offsets = settings.reminder_offsets_minutes;
                offsets.sort_unstable();
                offsets.dedup();
                offsets.truncate(MAX_VALARMS_PER_EVENT);
                Ok(offsets)
            }
        }
    }

    /// Look up a calendar entity by id, ignoring owner. Used by `render` and
    /// `FrozenIcsService::regenerate`. Soft-deleted calendars return `NotFound`.
    async fn load_calendar_by_id(&self, calendar_id: i32) -> ServerResult<CalendarEntity> {
        let row = sqlx::query!(
            r#"SELECT id, name, language as "language: LanguageEntity",
                      user_id, subscription_token, created_at, updated_at,
                      event_style, frozen_subscribe_ics, meta_version
               FROM calendars
               WHERE id = $1 AND deleted_at IS NULL"#,
            calendar_id,
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(r) = row else {
            return Err(Error::NotFound);
        };

        let item_ids: Vec<i32> = sqlx::query_scalar!(
            "SELECT item_id FROM calendar_items WHERE calendar_id = $1",
            calendar_id,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(CalendarEntity {
            id: r.id,
            name: r.name,
            language: r.language,
            user_id: r.user_id,
            subscription_token: r.subscription_token,
            item_ids,
            created_at: r.created_at,
            updated_at: r.updated_at,
            event_style: r.event_style,
            frozen_subscribe_ics: r.frozen_subscribe_ics,
            meta_version: r.meta_version,
        })
    }
}

/// Strip CR and LF characters from a calendar name so it cannot inject
/// arbitrary lines into a line-oriented output (ICS body,
/// `Content-Disposition` header, etc.). Each CR or LF is replaced with a
/// single ASCII space; consecutive whitespace is then collapsed.
///
/// Callers are responsible for downstream value-level escaping (commas,
/// semicolons, backslashes per RFC 5545) — that is handled by the
/// `icalendar` crate's typed builders, not this helper.
#[must_use]
pub(crate) fn sanitize_name_for_line_protocol(name: &str) -> String {
    name.chars()
        .map(|c| if c == '\r' || c == '\n' { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Build a safe filename for `Content-Disposition: attachment; filename="…"`.
///
/// Strips CR/LF (replaces with space), replaces double-quotes and
/// backslashes (the only characters that break out of the RFC 7230
/// quoted-string production), collapses runs of whitespace into underscores,
/// and lowercases. Non-ASCII characters are kept as-is; most modern user
/// agents accept UTF-8 in the legacy quoted form.
#[must_use]
pub(crate) fn build_attachment_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '\r' | '\n' => ' ',
            '"' | '\\' => '_',
            _ => c,
        })
        .collect();
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join("_");
    format!("{}.ics", collapsed.to_lowercase())
}

fn empty_calendar_ics(name: &str) -> String {
    let safe_name = sanitize_name_for_line_protocol(name);
    format!(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//anime-calendar//EN\r\nNAME:{safe_name}\r\nEND:VCALENDAR\r\n",
    )
}

/// Default per-event duration for `event_style == "timed"`. Anime episodes
/// are nominally 24 minutes; this matches the "airing" heuristic used in
/// `count_airing` on the calendar controller.
const TIMED_EVENT_DURATION_MIN: i64 = 24;

/// Render a `common::Calendar` to icalendar `Ics`. Each event receives one
/// VALARM per entry in `valarm_offsets` (in emission order). When
/// `event_style == "timed"` and the schedule has a real airing time, emits a
/// timed event with a 24-minute duration; otherwise falls back to all-day.
#[allow(deprecated, clippy::cast_possible_wrap)]
fn render_common_calendar(
    calendar: &CommonCalendar,
    event_style: &str,
    valarm_offsets: &[i32],
) -> Ics {
    let want_timed = event_style == "timed";

    let events: Vec<Event> = calendar
        .items
        .iter()
        .flat_map(|item| {
            debug!("Processing item {}", item.title.english);
            item.airing_schedule
                .iter()
                .map(|episode| {
                    debug!(
                        "Processing {} episode {} airing at {}",
                        item.title.english,
                        episode.episode,
                        episode.airing_at.to_int()
                    );
                    let summary = format!(
                        "{} - Episode {}",
                        match calendar.language {
                            Language::English => item.title.english.clone(),
                            Language::Romaji => item.title.romaji.clone(),
                            Language::Native => item.title.native.clone(),
                        },
                        episode.episode
                    );
                    let mut event = Event::new();

                    let airing_secs = episode.airing_at.to_int() as i64;
                    let dt = DateTime::from_timestamp(airing_secs, 0).unwrap_or_default();

                    if want_timed && airing_secs > 0 {
                        event.starts(dt);
                        event.ends(dt + Duration::minutes(TIMED_EVENT_DURATION_MIN));
                    } else {
                        event.all_day(dt.naive_utc().date());
                    }

                    event.summary(&summary);
                    for &offset_minutes in valarm_offsets {
                        event.alarm(Alarm::display(
                            &summary,
                            -Duration::minutes(i64::from(offset_minutes)),
                        ));
                    }
                    event.done()
                })
                .collect::<Vec<Event>>()
        })
        .collect();

    let mut ics = Ics::new();
    let sanitized_name = sanitize_name_for_line_protocol(&calendar.name);
    let mut ics = ics.name(&sanitized_name);
    for event in events {
        ics = ics.push(event);
    }
    ics.done()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::Cache;
    use crate::config::server::{CacheConfig, LimitsConfig};
    use crate::mappers::anilist::Anilist;
    use crate::mappers::subscription::SubscriptionMapper;
    use crate::services::show_count::ShowCountService;
    use chrono::NaiveDateTime;
    use common::id::Id;
    use common::item::{Item, Type as ItemType};
    use common::language::Language as CommonLanguage;
    use common::media_cover::MediaCover;
    use common::schedule::Schedule;
    use common::timestamp::Timestamp;
    use common::title::Title;

    /// Build an `IcsExportService` wired to the given pool, with a real
    /// `CachedAnilist` backed by `Cache::for_tests` and a default
    /// `CacheConfig`. Used by tests that don't need to exercise upstream
    /// `AniList` — empty calendars short-circuit before any network call.
    async fn for_tests(pool: PgPool) -> IcsExportService {
        let cached = CachedAnilist::new(
            Anilist::default(),
            Cache::for_tests().await,
            &CacheConfig::default(),
        );
        let user_settings_mapper = UserSettingsMapper::from_pool(pool.clone());
        let entitlement = EntitlementService::new(
            SubscriptionMapper::from_pool(pool.clone()),
            ShowCountService::new(pool.clone()),
            &LimitsConfig::default(),
        );
        IcsExportService::new(pool, cached, user_settings_mapper, entitlement)
    }

    async fn seed_user(pool: &PgPool) -> i32 {
        let n: u64 = rand::random();
        let username = format!("ics_{n}");
        let email = format!("ics_{n}@test.com");
        sqlx::query_scalar(
            "INSERT INTO users (username, email, password_hash) \
             VALUES ($1, $2, 'hash') RETURNING id",
        )
        .bind(username)
        .bind(email)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn seed_calendar(pool: &PgPool, user_id: i32, name: &str, deleted: bool) -> i32 {
        let n: u64 = rand::random();
        let token = format!("tok-{n}");
        if deleted {
            sqlx::query_scalar(
                "INSERT INTO calendars (name, language, user_id, subscription_token, deleted_at) \
                 VALUES ($1, 'english'::language, $2, $3, NOW()) RETURNING id",
            )
            .bind(name)
            .bind(user_id)
            .bind(token)
            .fetch_one(pool)
            .await
            .unwrap()
        } else {
            sqlx::query_scalar(
                "INSERT INTO calendars (name, language, user_id, subscription_token) \
                 VALUES ($1, 'english'::language, $2, $3) RETURNING id",
            )
            .bind(name)
            .bind(user_id)
            .bind(token)
            .fetch_one(pool)
            .await
            .unwrap()
        }
    }

    async fn make_paid(pool: &PgPool, user_id: i32) {
        use chrono::{Duration, Utc};
        let now = Utc::now().naive_utc();
        sqlx::query(
            "INSERT INTO subscriptions \
             (user_id, tier, status, stripe_customer_id, stripe_subscription_id, \
              stripe_price_id, current_period_start, current_period_end, \
              cancel_at_period_end) \
             VALUES ($1, 'paid', 'active', $2, $3, 'price_test', $4, $5, false)",
        )
        .bind(user_id)
        .bind(format!("cus_test_{user_id}"))
        .bind(format!("sub_test_{user_id}"))
        .bind(now)
        .bind(now + Duration::days(30))
        .execute(pool)
        .await
        .unwrap();
    }

    async fn set_reminder_offsets(pool: &PgPool, user_id: i32, offsets: Vec<i32>) {
        sqlx::query(
            "INSERT INTO user_settings (user_id, reminder_offsets_minutes) \
             VALUES ($1, $2) \
             ON CONFLICT (user_id) DO UPDATE SET reminder_offsets_minutes = EXCLUDED.reminder_offsets_minutes",
        )
        .bind(user_id)
        .bind(&offsets)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn cleanup_user(pool: &PgPool, user_id: i32) {
        sqlx::query(
            "DELETE FROM calendar_items WHERE calendar_id IN \
             (SELECT id FROM calendars WHERE user_id = $1)",
        )
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query("DELETE FROM calendars WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM subscriptions WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM user_settings WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
    }

    /// Build a `CommonCalendar` with one item containing one episode airing
    /// at the given UNIX timestamp. Used by the renderer-level unit tests
    /// (path (c) in the brief — pure logic, no DB / `AniList` plumbing).
    fn synthetic_calendar(airing_at: i64) -> CommonCalendar {
        let item = Item {
            id: Id::new(1).unwrap(),
            id_mal: None,
            title: Title {
                english: "Test Show".to_owned(),
                romaji: "Test Show".to_owned(),
                native: "Test Show".to_owned(),
            },
            airing_schedule: vec![Schedule {
                id: Id::new(1).unwrap(),
                airing_at: Timestamp::new(airing_at).expect("positive airing_at"),
                episode: 1,
                media_id: Id::new(1),
            }],
            episode_duration: 24,
            media_type: ItemType::Anime,
            cover_image: MediaCover {
                extra_large: String::new(),
                large: String::new(),
                medium: String::new(),
                color: String::new(),
            },
            banner_image: String::new(),
            recommendations: vec![],
        };
        CommonCalendar {
            id: Id::new(1).unwrap(),
            items: vec![item],
            language: CommonLanguage::English,
            name: "Test Calendar".to_owned(),
            created_at: NaiveDateTime::default(),
            updated_at: NaiveDateTime::default(),
        }
    }

    // ─── render_common_calendar (pure unit tests) ────────────────────────────

    #[test]
    fn pro_three_offsets_emits_three_valarms() {
        let cal = synthetic_calendar(1_700_000_000);
        let out = format!(
            "{}",
            render_common_calendar(&cal, "timed", &[30, 60, 1440])
        );
        assert_eq!(
            out.matches("BEGIN:VALARM").count(),
            3,
            "expected 3 VALARMs in output:\n{out}"
        );
        // icalendar 0.17 emits Duration triggers in seconds form: 30 min →
        // -PT1800S, 60 min → -PT3600S, 1440 min (1 day) → -PT86400S. Assert
        // all three are present in the output.
        assert!(
            out.contains("TRIGGER:-PT1800S"),
            "missing 30-min trigger:\n{out}"
        );
        assert!(
            out.contains("TRIGGER:-PT3600S"),
            "missing 60-min trigger:\n{out}"
        );
        assert!(
            out.contains("TRIGGER:-PT86400S"),
            "missing 1440-min trigger:\n{out}"
        );
    }

    #[test]
    fn pro_zero_offsets_emits_zero_valarms() {
        let cal = synthetic_calendar(1_700_000_000);
        let out = format!("{}", render_common_calendar(&cal, "timed", &[]));
        assert!(
            !out.contains("BEGIN:VALARM"),
            "expected no VALARMs:\n{out}"
        );
    }

    #[test]
    fn free_renders_single_30min_valarm_regardless_of_passed_offsets() {
        // Simulates the renderer being given Free-tier offsets after the tier
        // branch resolved them — i.e. exactly `[30]`, regardless of the
        // settings array.
        let cal = synthetic_calendar(1_700_000_000);
        let out = format!("{}", render_common_calendar(&cal, "timed", &[30]));
        assert_eq!(
            out.matches("BEGIN:VALARM").count(),
            1,
            "expected exactly one VALARM:\n{out}"
        );
        // 30 min → 1800 seconds in icalendar 0.17's Duration emission.
        assert!(
            out.contains("TRIGGER:-PT1800S"),
            "missing 30-min trigger:\n{out}"
        );
    }

    #[test]
    fn event_style_all_day_emits_value_date() {
        let cal = synthetic_calendar(1_700_000_000);
        let out = format!("{}", render_common_calendar(&cal, "all_day", &[30]));
        assert!(
            out.contains("DTSTART;VALUE=DATE"),
            "expected all-day DTSTART;VALUE=DATE:\n{out}"
        );
    }

    #[test]
    fn event_style_timed_emits_dtstart_with_time() {
        let cal = synthetic_calendar(1_700_000_000);
        let out = format!("{}", render_common_calendar(&cal, "timed", &[30]));
        // DTSTART line should not have ;VALUE=DATE and should include a 'T'
        // separator (datetime form: YYYYMMDDTHHMMSSZ).
        assert!(
            !out.contains("DTSTART;VALUE=DATE"),
            "timed event should not be VALUE=DATE:\n{out}"
        );
        // Find the DTSTART line and assert it has datetime format.
        let dtstart_line = out
            .lines()
            .find(|l| l.starts_with("DTSTART"))
            .unwrap_or_else(|| panic!("no DTSTART line:\n{out}"));
        assert!(
            dtstart_line.contains('T'),
            "DTSTART missing time component: {dtstart_line}"
        );
    }

    // ─── render() integration tests ──────────────────────────────────────────

    #[tokio::test]
    async fn render_empty_calendar_returns_minimal_ics() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = seed_user(&pool).await;
        let cal_id = seed_calendar(&pool, user_id, "Empty", false).await;

        let svc = for_tests(pool.clone()).await;
        let out = svc.render(cal_id).await.expect("render");

        assert!(out.contains("BEGIN:VCALENDAR"));
        assert!(out.contains("END:VCALENDAR"));
        assert!(!out.contains("BEGIN:VEVENT"));

        cleanup_user(&pool, user_id).await;
    }

    #[tokio::test]
    async fn render_nonexistent_calendar_returns_not_found() {
        let pool = crate::test_helpers::test_pool().await;
        let svc = for_tests(pool).await;
        // Pick an id that's astronomically unlikely to collide with concurrent
        // tests using the shared pool. `% 1000` keeps the value < i32::MAX, so
        // the cast cannot wrap.
        let phantom_id = i32::MAX - i32::try_from(rand::random::<u32>() % 1000).unwrap();
        let result = svc.render(phantom_id).await;
        assert!(matches!(result, Err(Error::NotFound)));
    }

    #[tokio::test]
    async fn render_soft_deleted_calendar_returns_not_found() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = seed_user(&pool).await;
        let cal_id = seed_calendar(&pool, user_id, "Gone", true).await;

        let svc = for_tests(pool.clone()).await;
        let result = svc.render(cal_id).await;
        assert!(matches!(result, Err(Error::NotFound)));

        cleanup_user(&pool, user_id).await;
    }

    /// Free-tier anti-bypass: even with Pro-shaped offsets stored in
    /// `user_settings`, the renderer must still emit exactly one 30-min
    /// VALARM. This exercises the tier resolution path inside `render` (via
    /// `resolve_valarm_offsets`).
    #[tokio::test]
    async fn render_free_user_ignores_stored_offsets_30min_only() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = seed_user(&pool).await;
        // Free tier — but with multi-offset settings written. The renderer
        // must NOT honour these for a free user.
        set_reminder_offsets(&pool, user_id, vec![60, 1440]).await;
        // Empty calendar would short-circuit before the tier path, so call
        // resolve_valarm_offsets directly via a synthetic render of a
        // hand-built CommonCalendar. We can't easily seed a calendar with
        // resolvable items (depends on AniList cache), but we can verify the
        // resolver result through the public-ish API.
        let svc = for_tests(pool.clone()).await;
        let offsets = svc
            .resolve_valarm_offsets(user_id)
            .await
            .expect("resolve offsets");
        assert_eq!(
            offsets,
            vec![30],
            "Free user must always resolve to single 30-min offset"
        );

        cleanup_user(&pool, user_id).await;
    }

    /// Pro-tier offsets are loaded from settings, deduped, sorted, and
    /// truncated.
    #[tokio::test]
    async fn render_pro_user_resolves_offsets_from_settings() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = seed_user(&pool).await;
        make_paid(&pool, user_id).await;
        set_reminder_offsets(&pool, user_id, vec![60, 30, 1440]).await;

        let svc = for_tests(pool.clone()).await;
        let offsets = svc
            .resolve_valarm_offsets(user_id)
            .await
            .expect("resolve offsets");
        assert_eq!(offsets, vec![30, 60, 1440], "expected sorted deduped");

        cleanup_user(&pool, user_id).await;
    }

    /// Defense-in-depth: a Pro user with a >5-entry array (somehow slipped
    /// past validation) still produces at most 5 VALARMs.
    #[tokio::test]
    async fn render_pro_user_truncates_excess_offsets() {
        let pool = crate::test_helpers::test_pool().await;
        let user_id = seed_user(&pool).await;
        make_paid(&pool, user_id).await;
        // Direct insert bypasses validation — simulates legacy data.
        set_reminder_offsets(&pool, user_id, vec![15, 30, 60, 120, 360, 720, 1440]).await;

        let svc = for_tests(pool.clone()).await;
        let offsets = svc
            .resolve_valarm_offsets(user_id)
            .await
            .expect("resolve offsets");
        assert_eq!(offsets.len(), MAX_VALARMS_PER_EVENT);

        cleanup_user(&pool, user_id).await;
    }

    // ─── sanitize_name_for_line_protocol ────────────────────────────────────

    #[test]
    fn sanitize_strips_cr() {
        assert_eq!(sanitize_name_for_line_protocol("a\rb"), "a b");
    }

    #[test]
    fn sanitize_strips_lf() {
        assert_eq!(sanitize_name_for_line_protocol("a\nb"), "a b");
    }

    #[test]
    fn sanitize_strips_crlf() {
        assert_eq!(sanitize_name_for_line_protocol("a\r\nb"), "a b");
    }

    #[test]
    fn sanitize_collapses_multiple_newlines() {
        assert_eq!(sanitize_name_for_line_protocol("a\nb\nc"), "a b c");
    }

    #[test]
    fn sanitize_passthrough_clean_name() {
        assert_eq!(sanitize_name_for_line_protocol("My Calendar"), "My Calendar");
    }

    #[test]
    fn sanitize_collapses_internal_whitespace() {
        // Multiple spaces that result from stripping are collapsed.
        assert_eq!(sanitize_name_for_line_protocol("a\r\n\r\nb"), "a b");
    }

    // ─── build_attachment_filename ───────────────────────────────────────────

    #[test]
    fn filename_no_special_chars() {
        assert_eq!(build_attachment_filename("My Calendar"), "my_calendar.ics");
    }

    #[test]
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    fn filename_strips_double_quote() {
        let out = build_attachment_filename("My \"Calendar\"");
        assert!(!out.contains('"'), "output must not contain double-quote: {out}");
        assert!(out.ends_with(".ics"), "output must end with .ics: {out}");
    }

    #[test]
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    fn filename_strips_backslash() {
        let out = build_attachment_filename("My\\Cal");
        assert!(!out.contains('\\'), "output must not contain backslash: {out}");
        assert!(out.ends_with(".ics"), "output must end with .ics: {out}");
    }

    #[test]
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    fn filename_strips_crlf() {
        let out = build_attachment_filename("My\r\nCalendar");
        assert!(!out.contains('\r'), "CR must be stripped: {out}");
        assert!(!out.contains('\n'), "LF must be stripped: {out}");
        assert!(out.ends_with(".ics"), "output must end with .ics: {out}");
    }

    #[test]
    fn filename_lowercased() {
        assert_eq!(build_attachment_filename("ABC"), "abc.ics");
    }

    // ─── empty_calendar_ics CRLF-injection guard ─────────────────────────────

    #[test]
    fn empty_calendar_ics_injection_does_not_add_second_vevent() {
        // Simulate a name whose raw value contains CRLF-injected ICS lines.
        // After sanitization the CR/LF are replaced with spaces so the injected
        // keywords are part of the NAME value on a single line — not a
        // standalone `BEGIN:VEVENT` line that calendar parsers would treat as a
        // real event block.
        let malicious_name = "Evil\r\nBEGIN:VEVENT\r\nSUMMARY:Injected\r\nEND:VEVENT\r\n";
        let out = empty_calendar_ics(malicious_name);

        // The critical invariant: no standalone `BEGIN:VEVENT` line exists.
        // A real injection would appear as a CRLF-terminated line starting
        // with "BEGIN:VEVENT". After sanitization the output has exactly one
        // line per `\r\n`, none of which is a bare `BEGIN:VEVENT`.
        let line_starts_vevent = out
            .split("\r\n")
            .any(|line| line.trim() == "BEGIN:VEVENT");
        assert!(
            !line_starts_vevent,
            "ICS output must not contain a standalone BEGIN:VEVENT line:\n{out}"
        );

        // The sanitized name should still be present in the output.
        assert!(
            out.contains("Evil"),
            "Sanitized calendar name prefix should appear:\n{out}"
        );
        // The whole output is a valid minimal calendar (starts and ends correctly).
        assert!(out.starts_with("BEGIN:VCALENDAR\r\n"));
        assert!(out.ends_with("END:VCALENDAR\r\n"));
    }

    // ─── render_common_calendar CRLF-injection guard (H-1 follow-up) ─────────

    #[test]
    fn render_common_calendar_injection_does_not_add_second_vevent() {
        // Simulate a calendar whose `name` field contains a CRLF-injected ICS
        // block. The `icalendar` crate's `.name()` builder handles RFC 5545
        // value-level escaping (commas, semicolons, backslashes) but does NOT
        // strip CR/LF — so without sanitization `X-WR-CALNAME` would contain
        // injected lines that calendar parsers treat as real property lines.
        let mut cal = synthetic_calendar(1_700_000_000);
        cal.name = "Evil\r\nBEGIN:VEVENT\r\nSUMMARY:Injected\r\nEND:VEVENT\r\nX-WR-CALNAME:"
            .to_owned();

        let out = format!("{}", render_common_calendar(&cal, "allday", &[]));

        // Critical invariant: no standalone BEGIN:VEVENT injected by the name.
        // The real calendar item contributes exactly one BEGIN:VEVENT; a second
        // one would mean the injection succeeded.
        let vevent_count = out.split("\r\n").filter(|l| *l == "BEGIN:VEVENT").count();
        assert_eq!(
            vevent_count,
            1,
            "expected exactly 1 BEGIN:VEVENT (the real event); \
             injection guard failed:\n{out}"
        );

        // The sanitized name prefix should still appear in the output.
        assert!(
            out.contains("Evil"),
            "Sanitized calendar name prefix should appear:\n{out}"
        );
    }
}
