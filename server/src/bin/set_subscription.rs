//! Dev-only CLI to set a user's subscription state for testing.
//!
//! Logging note (L-17): this binary uses `eprintln!`/`println!` deliberately —
//! a CLI's job is to write human-readable output to stderr/stdout, so the
//! server convention of routing through `log::error!` does not apply here.
//!
//! Usage:
//!   `set_subscription` --email <email> <state>
//!   `set_subscription` --user-id <id>  <state>
//!   `set_subscription` --email <email> --reconcile-from-stripe
//!
//! States:
//!   free                    Delete all subscription rows for the user.
//!   trialing                status=trialing, `period_end=now+14d`, `trial_end=period_end`.
//!   active                  status=active,   `period_end=now+30d`.
//!   past-due                `status=past_due`, `period_end=now+3d`.
//!   cancel-at-period-end    status=active,   `cancel_at_period_end=true`, `period_end=now+10d`.
//!   canceled-expired        status=canceled, period_end=now-1d.
//!   incomplete              status=incomplete, `period_end=now+30d`.
//!
//! `--reconcile-from-stripe` pulls Stripe truth for the user's active
//! subscription and applies the same drift correction the hourly loop would.
//! Useful for debugging webhook delivery gaps or hand-checking a customer
//! after the fact.
//!
//! Safety: refuses to run when `APP_ENV=production` unless
//! `--i-know-this-is-production` is passed. That override exists because
//! comping access is a legitimate production need — the owner's own account,
//! and friends given free access — and the alternative workaround (setting
//! `environment = "staging"`) also disarms the `cookie_secure` check that
//! `Config::validate` only enforces under `"production"`.
//!
//! Note that `APP_ENV` records *intent*, not which database is on the other
//! end of the socket, and it is usually unset when running from a laptop
//! against a Cloud SQL proxy. The target host/database is therefore printed
//! before any mutation, whichever way the guard goes — that is the line to
//! read before pressing on.
//!
//! Reads database.toml from CWD, honouring `url` (a managed provider's full
//! connection string) the same way the server does.
//! `--reconcile-from-stripe` additionally reads config.toml for the Stripe
//! secret key. This bin is excluded from the production Docker image (the
//! image copies only `target/release/server`), so production use means running
//! it locally against the production DSN.

use std::{env, process::ExitCode};

use chrono::{Duration, NaiveDateTime, Utc};
use server::config::database::Database as DatabaseConfig;
use server::config::server::Server as ServerConfig;
use server::mappers::subscription::SubscriptionMapper;
use server::services::reconcile::{LiveStripeFetcher, reconcile_for_user};
use sqlx::PgPool;

#[derive(Debug)]
enum State {
    Free,
    Trialing,
    Active,
    PastDue,
    CancelAtPeriodEnd,
    CanceledExpired,
    Incomplete,
}

impl State {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "free" => Self::Free,
            "trialing" => Self::Trialing,
            "active" => Self::Active,
            "past-due" => Self::PastDue,
            "cancel-at-period-end" => Self::CancelAtPeriodEnd,
            "canceled-expired" => Self::CanceledExpired,
            "incomplete" => Self::Incomplete,
            _ => return None,
        })
    }
}

#[derive(Debug)]
enum UserSelector {
    Email(String),
    UserId(i32),
}

fn print_usage() {
    eprintln!(
        "Usage:\n  \
           set_subscription (--email <email> | --user-id <id>) <state>\n  \
           set_subscription (--email <email> | --user-id <id>) --reconcile-from-stripe\n\
         States: free | trialing | active | past-due | cancel-at-period-end | \
         canceled-expired | incomplete\n\
         \n\
         Flags:\n  \
           --i-know-this-is-production  Override the APP_ENV=production guard.\n  \
           \x20                         Needed to comp an account on the live\n  \
           \x20                         system. The target database is printed\n  \
           \x20                         before any change either way."
    );
}

#[derive(Debug)]
enum Mode {
    SetState(State),
    ReconcileFromStripe,
}

/// Returns the selector, the mode, and whether the production guard was
/// explicitly overridden.
fn parse_args() -> Result<(UserSelector, Mode, bool), String> {
    let mut args = env::args().skip(1);
    let mut selector: Option<UserSelector> = None;
    let mut state: Option<State> = None;
    let mut reconcile = false;
    let mut allow_production = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--email" => {
                let v = args.next().ok_or("--email requires a value")?;
                selector = Some(UserSelector::Email(v));
            }
            "--user-id" => {
                let v = args.next().ok_or("--user-id requires a value")?;
                let id: i32 = v.parse().map_err(|_| "--user-id must be an integer")?;
                selector = Some(UserSelector::UserId(id));
            }
            "--reconcile-from-stripe" => {
                reconcile = true;
            }
            "--i-know-this-is-production" => {
                allow_production = true;
            }
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            other => {
                if state.is_some() {
                    return Err(format!("unexpected argument: {other}"));
                }
                state = Some(State::parse(other).ok_or_else(|| format!("unknown state: {other}"))?);
            }
        }
    }

    let selector = selector.ok_or("missing --email or --user-id")?;
    let mode = match (reconcile, state) {
        (true, None) => Mode::ReconcileFromStripe,
        (true, Some(_)) => {
            return Err("--reconcile-from-stripe is exclusive with a <state> argument".into());
        }
        (false, Some(s)) => Mode::SetState(s),
        (false, None) => return Err("missing state argument or --reconcile-from-stripe".into()),
    };
    Ok((selector, mode, allow_production))
}

async fn resolve_user_id(pool: &PgPool, selector: &UserSelector) -> Result<i32, String> {
    match selector {
        UserSelector::UserId(id) => {
            let exists: Option<i32> = sqlx::query_scalar("SELECT id FROM users WHERE id = $1")
                .bind(id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("db error: {e}"))?;
            exists.ok_or_else(|| format!("no user with id {id}"))
        }
        UserSelector::Email(email) => {
            let id: Option<i32> = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
                .bind(email)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("db error: {e}"))?;
            id.ok_or_else(|| format!("no user with email {email}"))
        }
    }
}

struct SubRow {
    status: &'static str,
    cancel_at_period_end: bool,
    period_start: NaiveDateTime,
    period_end: NaiveDateTime,
    trial_end: Option<NaiveDateTime>,
}

fn build_row(state: &State) -> Option<SubRow> {
    let now = Utc::now().naive_utc();
    let period_start = now - Duration::days(1);
    Some(match state {
        State::Free => return None,
        State::Trialing => {
            let pe = now + Duration::days(14);
            SubRow {
                status: "trialing",
                cancel_at_period_end: false,
                period_start,
                period_end: pe,
                trial_end: Some(pe),
            }
        }
        State::Active => SubRow {
            status: "active",
            cancel_at_period_end: false,
            period_start,
            period_end: now + Duration::days(30),
            trial_end: None,
        },
        State::PastDue => SubRow {
            status: "past_due",
            cancel_at_period_end: false,
            period_start,
            period_end: now + Duration::days(3),
            trial_end: None,
        },
        State::CancelAtPeriodEnd => SubRow {
            status: "active",
            cancel_at_period_end: true,
            period_start,
            period_end: now + Duration::days(10),
            trial_end: None,
        },
        State::CanceledExpired => SubRow {
            status: "canceled",
            cancel_at_period_end: false,
            period_start: now - Duration::days(31),
            period_end: now - Duration::days(1),
            trial_end: None,
        },
        State::Incomplete => SubRow {
            status: "incomplete",
            cancel_at_period_end: false,
            period_start,
            period_end: now + Duration::days(30),
            trial_end: None,
        },
    })
}

async fn apply(pool: &PgPool, user_id: i32, state: &State) -> Result<String, String> {
    // Always wipe existing rows first — keeps state transitions clean.
    sqlx::query("DELETE FROM subscriptions WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| format!("delete failed: {e}"))?;

    let Some(row) = build_row(state) else {
        return Ok(format!("user {user_id}: cleared (free tier)"));
    };

    // Synthetic Stripe identifiers — clearly fake so they're never confused
    // with real Stripe data if someone misruns this against prod.
    let n: u64 = rand::random();
    let stripe_customer_id = format!("cus_dev_{user_id}_{n}");
    let stripe_subscription_id = format!("sub_dev_{user_id}_{n}");
    let stripe_price_id = "price_dev_local";

    sqlx::query(
        "INSERT INTO subscriptions \
         (user_id, tier, status, stripe_customer_id, stripe_subscription_id, \
          stripe_price_id, current_period_start, current_period_end, trial_end, \
          cancel_at_period_end) \
         VALUES ($1, 'paid', $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(user_id)
    .bind(row.status)
    .bind(&stripe_customer_id)
    .bind(&stripe_subscription_id)
    .bind(stripe_price_id)
    .bind(row.period_start)
    .bind(row.period_end)
    .bind(row.trial_end)
    .bind(row.cancel_at_period_end)
    .execute(pool)
    .await
    .map_err(|e| format!("insert failed: {e}"))?;

    Ok(format!(
        "user {user_id}: status={} cancel_at_period_end={} period_end={} trial_end={:?}",
        row.status, row.cancel_at_period_end, row.period_end, row.trial_end
    ))
}

async fn apply_reconcile(pool: &PgPool, user_id: i32) -> Result<String, String> {
    use secrecy::ExposeSecret;

    let cfg = ServerConfig::new().map_err(|e| format!("could not load config.toml: {e}"))?;
    if !cfg.stripe.is_configured() {
        return Err("stripe is not configured in config.toml".into());
    }
    let client = stripe::Client::new(cfg.stripe.secret_key.expose_secret().to_string());
    let fetcher = LiveStripeFetcher::new(client);
    let mapper = SubscriptionMapper::from_pool(pool.clone());

    match reconcile_for_user(&mapper, &fetcher, user_id).await? {
        None => Ok(format!(
            "user {user_id}: no active subscription row to reconcile"
        )),
        Some(true) => Ok(format!("user {user_id}: drift corrected from Stripe")),
        Some(false) => Ok(format!(
            "user {user_id}: no drift detected (local matches Stripe, or fresher)"
        )),
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    use secrecy::ExposeSecret;

    let (selector, mode, allow_production) = match parse_args() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            print_usage();
            return ExitCode::from(2);
        }
    };

    // Guard: refuse under a production APP_ENV unless the operator says so
    // explicitly on the command line.
    //
    // The override exists because comping access is a legitimate production
    // need — the owner's own account, and friends given free access — and the
    // previous workaround was to set `environment = "staging"`, which also
    // disarms the `cookie_secure` check that `Config::validate` enforces only
    // under "production". Trading a real safety check for a comp mechanism is
    // the wrong trade; an explicit flag keeps the guard on by default and
    // makes bypassing it a visible, deliberate act that shows up in shell
    // history.
    //
    // Note what this guard can and cannot see: `APP_ENV` states *intent*. It
    // says nothing about which database is on the other end of the socket, and
    // it is typically unset when running from a laptop against a Cloud SQL
    // proxy — the exact case you would most want caught. That is why the
    // target is printed below regardless.
    if let Ok(env_name) = env::var("APP_ENV")
        && (env_name.eq_ignore_ascii_case("production") || env_name.eq_ignore_ascii_case("prod"))
    {
        if allow_production {
            eprintln!("warning: APP_ENV={env_name} and --i-know-this-is-production was passed.");
        } else {
            eprintln!(
                "refusing to run: APP_ENV={env_name}.\n\
                 Pass --i-know-this-is-production to override (used for comping accounts)."
            );
            return ExitCode::from(2);
        }
    }

    let cfg = match DatabaseConfig::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("could not load database.toml: {e}");
            return ExitCode::from(1);
        }
    };

    // Resolve the DSN through the same helper the server uses, so `url` (a
    // managed provider's full connection string) is honoured rather than
    // silently ignored in favour of the individual fields.
    let url = server::mappers::database::connection_string(&cfg);

    // Always say where this is pointed before touching anything. This is the
    // signal that actually corresponds to reality, unlike APP_ENV.
    eprintln!(
        "target: {}",
        server::mappers::database::describe_target(&cfg)
    );

    let pool = match PgPool::connect(url.expose_secret()).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("db connect failed: {e}");
            return ExitCode::from(1);
        }
    };

    let user_id = match resolve_user_id(&pool, &selector).await {
        Ok(id) => id,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };

    let result = match mode {
        Mode::SetState(state) => apply(&pool, user_id, &state).await,
        Mode::ReconcileFromStripe => apply_reconcile(&pool, user_id).await,
    };

    match result {
        Ok(msg) => {
            println!("{msg}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
