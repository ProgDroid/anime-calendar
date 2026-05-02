use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, fmt, str::FromStr};

/// Local mirror of a Stripe `Subscription`. Webhook handlers + the reconcile
/// loop both write to this row; the entitlement service reads it to gate
/// Pro-only features.
#[derive(Clone, Debug, Deserialize, Serialize, sqlx::FromRow)]
pub struct Subscription {
    pub id: i32,
    pub user_id: i32,
    pub tier: String,
    pub status: String,
    pub stripe_customer_id: String,
    pub stripe_subscription_id: String,
    pub stripe_price_id: String,
    pub current_period_start: NaiveDateTime,
    pub current_period_end: NaiveDateTime,
    pub trial_end: Option<NaiveDateTime>,
    pub cancel_at_period_end: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Effective access tier for a user. `Free` is the default whenever no active
/// subscription row exists.
///
/// Stored as `VARCHAR` rather than a Postgres ENUM so future tiers (e.g.
/// `Studio`) can be added in code-only migrations without DDL churn.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    #[default]
    Free,
    Paid,
}

impl Tier {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Paid => "paid",
        }
    }
}

impl FromStr for Tier {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "paid" => Self::Paid,
            _ => Self::Free,
        })
    }
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stripe subscription status, mirrored 1:1.
///
/// "Live access" is granted for `Trialing | Active | PastDue` while
/// `current_period_end > now()`. `PastDue` keeps access during dunning so a
/// failed renewal doesn't immediately lock the user out — Stripe will retry
/// the charge before flipping to `Unpaid` or `Canceled`.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Trialing,
    Active,
    PastDue,
    Canceled,
    Incomplete,
    IncompleteExpired,
    Unpaid,
    Paused,
}

impl Status {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Trialing => "trialing",
            Self::Active => "active",
            Self::PastDue => "past_due",
            Self::Canceled => "canceled",
            Self::Incomplete => "incomplete",
            Self::IncompleteExpired => "incomplete_expired",
            Self::Unpaid => "unpaid",
            Self::Paused => "paused",
        }
    }

    /// Returns true for statuses that grant the user live entitlement, subject
    /// to `current_period_end > now()`. The mapper's WHERE clause must agree
    /// with this set.
    #[must_use]
    pub const fn grants_access(self) -> bool {
        matches!(self, Self::Trialing | Self::Active | Self::PastDue)
    }
}

impl FromStr for Status {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "trialing" => Self::Trialing,
            "active" => Self::Active,
            "past_due" => Self::PastDue,
            "incomplete" => Self::Incomplete,
            "incomplete_expired" => Self::IncompleteExpired,
            "unpaid" => Self::Unpaid,
            "paused" => Self::Paused,
            _ => Self::Canceled,
        })
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Effective entitlement view used by the API + frontend. Captures both the
/// tier (`free` / `paid`) and the contextual metadata the UI needs to render
/// the Subscription tab without a second query.
#[derive(Clone, Debug, Serialize)]
pub struct Entitlement {
    pub tier: Tier,
    pub status: Option<String>,
    pub current_period_end: Option<NaiveDateTime>,
    pub cancel_at_period_end: bool,
    pub trial_end: Option<NaiveDateTime>,
}

impl Entitlement {
    #[must_use]
    pub const fn free() -> Self {
        Self {
            tier: Tier::Free,
            status: None,
            current_period_end: None,
            cancel_at_period_end: false,
            trial_end: None,
        }
    }

    #[must_use]
    pub fn from_subscription(sub: &Subscription) -> Self {
        Self {
            tier: Tier::from_str(&sub.tier).unwrap_or(Tier::Free),
            status: Some(sub.status.clone()),
            current_period_end: Some(sub.current_period_end),
            cancel_at_period_end: sub.cancel_at_period_end,
            trial_end: sub.trial_end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_round_trip() {
        assert_eq!(Tier::from_str("paid").unwrap(), Tier::Paid);
        assert_eq!(Tier::from_str("free").unwrap(), Tier::Free);
        assert_eq!(Tier::from_str("garbage").unwrap(), Tier::Free);
        assert_eq!(Tier::Paid.as_str(), "paid");
    }

    #[test]
    fn status_grants_access_matches_mapper_where_clause() {
        assert!(Status::Trialing.grants_access());
        assert!(Status::Active.grants_access());
        assert!(Status::PastDue.grants_access());
        assert!(!Status::Canceled.grants_access());
        assert!(!Status::Incomplete.grants_access());
        assert!(!Status::IncompleteExpired.grants_access());
        assert!(!Status::Unpaid.grants_access());
        assert!(!Status::Paused.grants_access());
    }
}
