use std::str::FromStr;

use crate::{
    ServerResult,
    entity::subscription::{Entitlement, Tier},
    mappers::subscription::SubscriptionMapper,
};

/// Effective-tier read service. Wraps `SubscriptionMapper` so handlers and
/// middleware never need to know the entitlement rules — just call
/// `effective_tier(user_id)` or `entitlement(user_id)`.
///
/// v1 deliberately does not cache: the read is a single indexed lookup and
/// caching introduces invalidation bugs the moment Stripe changes a sub.
#[derive(Clone)]
pub struct EntitlementService {
    mapper: SubscriptionMapper,
}

impl EntitlementService {
    #[must_use]
    pub const fn new(mapper: SubscriptionMapper) -> Self {
        Self { mapper }
    }

    /// Return just the tier — for fast gates that only branch on free/paid.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn effective_tier(&self, user_id: i32) -> ServerResult<Tier> {
        let sub = self.mapper.find_active_for_user(user_id).await?;
        Ok(sub.as_ref().map_or(Tier::Free, |s| {
            Tier::from_str(&s.tier).unwrap_or(Tier::Free)
        }))
    }

    /// Return the full entitlement view — for the Subscription tab and
    /// anywhere the UI needs status / period / trial context.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn entitlement(&self, user_id: i32) -> ServerResult<Entitlement> {
        let sub = self.mapper.find_active_for_user(user_id).await?;
        Ok(sub
            .as_ref()
            .map_or_else(Entitlement::free, Entitlement::from_subscription))
    }
}
