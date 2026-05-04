use chrono::{DateTime, Duration};
use common::calendar::Calendar as CommonCalendar;
use common::id::Id;
use common::item::AnimeDataSource as _;
use common::language::Language;
use icalendar::{Alarm, Calendar as Ics, Component as _, Event, EventLike as _};
use log::info;
use sqlx::PgPool;

use crate::entity::calendar::{Calendar as CalendarEntity, Language as LanguageEntity};
use crate::error::Error;
use crate::services::cached_anilist::CachedAnilist;
use crate::ServerResult;

/// Renders a calendar to a complete .ics document string.
///
/// Phase 1: always emits all-day events with a single hardcoded 30-minute
/// `VALARM`. Phase 2 will:
/// - honour `calendars.event_style` to switch between timed (`DTSTART:datetime`)
///   and all-day (`DTSTART;VALUE=DATE`) shapes
/// - emit one `VALARM` per stored offset for Pro users (capped at 5)
/// - keep emitting the single 30-minute alarm for Free users regardless of
///   the stored offsets (server-side anti-bypass)
#[derive(Clone)]
pub struct IcsExportService {
    pool: PgPool,
    cached_anilist: CachedAnilist,
}

impl IcsExportService {
    #[must_use]
    pub const fn new(pool: PgPool, cached_anilist: CachedAnilist) -> Self {
        Self {
            pool,
            cached_anilist,
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

        Ok(format!("{}", render_common_calendar(&common_cal)))
    }

    /// Look up a calendar entity by id, ignoring owner. Used by `render` and
    /// `FrozenIcsService::regenerate`. Soft-deleted calendars return `NotFound`.
    async fn load_calendar_by_id(&self, calendar_id: i32) -> ServerResult<CalendarEntity> {
        let row = sqlx::query!(
            r#"SELECT id, name, language as "language: LanguageEntity",
                      user_id, subscription_token, created_at, updated_at,
                      event_style, frozen_subscribe_ics
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
        })
    }
}

fn empty_calendar_ics(name: &str) -> String {
    format!(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//anime-calendar//EN\r\nNAME:{name}\r\nEND:VCALENDAR\r\n",
    )
}

/// Render a `common::Calendar` to icalendar `Ics`. Adds a single 30-minute
/// VALARM per event (Phase 1 baseline; Phase 2 replaces this with per-tier
/// per-user offset logic).
#[allow(deprecated, clippy::cast_possible_wrap)]
fn render_common_calendar(calendar: &CommonCalendar) -> Ics {
    let events: Vec<Event> = calendar
        .items
        .iter()
        .flat_map(|item| {
            info!("Processing item {}", item.title.english);
            item.airing_schedule
                .iter()
                .map(|episode| {
                    info!(
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
                    event.all_day(
                        DateTime::from_timestamp(episode.airing_at.to_int() as i64, 0)
                            .unwrap_or_default()
                            .naive_utc()
                            .date(),
                    );
                    event.summary(&summary);
                    // Phase 1 baseline: single 30-min reminder. Phase 2 swaps
                    // this for per-tier per-user offset emission. The renderer
                    // owns this anti-bypass invariant for Free users — never
                    // trust a stored offsets array here.
                    event.alarm(Alarm::display(&summary, -Duration::minutes(30)));
                    event.done()
                })
                .collect::<Vec<Event>>()
        })
        .collect();

    let mut ics = Ics::new();
    let mut ics = ics.name(&calendar.name);
    for event in events {
        ics = ics.push(event);
    }
    ics.done()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::Cache;
    use crate::config::server::CacheConfig;
    use crate::mappers::anilist::Anilist;

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
        IcsExportService::new(pool, cached)
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
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
    }

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
}
