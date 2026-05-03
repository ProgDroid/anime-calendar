//! Idempotency key store for Stripe webhook events.
//!
//! Every Stripe webhook event has a unique `stripe_event_id`. The webhook
//! handler attempts to insert the id with `ON CONFLICT DO NOTHING`; if the
//! insert affects zero rows the event has already been processed and the
//! handler short-circuits with 200 (Stripe will then stop retrying).
//!
//! The mapper exposes `record_first_time` for the simple "did we see this
//! before?" check, and `record_first_time_in_tx` for use inside an explicit
//! sqlx transaction so the idempotency insert and the subsequent business
//! writes commit or roll back atomically.

use crate::{
    ServerResult, config::database::Database as DatabaseConfig, mappers::database::Database,
};

#[derive(Clone)]
pub struct StripeEventMapper {
    db: Database,
}

impl StripeEventMapper {
    /// # Errors
    /// Fails if the database connection cannot be established.
    pub async fn new(config: DatabaseConfig) -> ServerResult<Self> {
        Ok(Self {
            db: Database::new(config).await?,
        })
    }

    /// Construct from a bare pool — for integration tests only.
    #[cfg(test)]
    #[must_use]
    pub const fn from_pool(pool: sqlx::PgPool) -> Self {
        Self {
            db: Database { pool },
        }
    }

    /// Borrow the underlying pool. The webhook controller uses this to open a
    /// transaction that spans the idempotency insert *and* the subscription
    /// upsert (both must commit atomically — see module docs).
    #[must_use]
    pub const fn pool(&self) -> &sqlx::PgPool {
        &self.db.pool
    }

    /// Insert the event id; returns `true` if this is the first time we've
    /// seen the event (and so the caller should process it), `false` if the
    /// event was already recorded (replay → skip processing).
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn record_first_time(
        &self,
        stripe_event_id: &str,
        event_type: &str,
    ) -> ServerResult<bool> {
        crate::metrics::db::timed("stripe_event.record_first_time", async {
            let inserted: Option<String> = sqlx::query_scalar!(
                "INSERT INTO stripe_events (stripe_event_id, event_type) \
                 VALUES ($1, $2) \
                 ON CONFLICT (stripe_event_id) DO NOTHING \
                 RETURNING stripe_event_id",
                stripe_event_id,
                event_type,
            )
            .fetch_optional(&self.db.pool)
            .await?;
            Ok(inserted.is_some())
        })
        .await
    }

    /// Same as `record_first_time` but participates in an existing
    /// transaction. Use when subsequent writes must commit atomically with
    /// the idempotency insert (rollback on business-logic failure leaves
    /// the event id absent so a Stripe retry will reprocess it).
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn record_first_time_in_tx(
        conn: &mut sqlx::PgConnection,
        stripe_event_id: &str,
        event_type: &str,
    ) -> ServerResult<bool> {
        let inserted: Option<String> = sqlx::query_scalar!(
            "INSERT INTO stripe_events (stripe_event_id, event_type) \
             VALUES ($1, $2) \
             ON CONFLICT (stripe_event_id) DO NOTHING \
             RETURNING stripe_event_id",
            stripe_event_id,
            event_type,
        )
        .fetch_optional(conn)
        .await?;
        Ok(inserted.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn record_first_time_returns_true_on_first_insert() {
        let mut tx = crate::test_helpers::test_tx().await;
        let n: u64 = rand::random();
        let id = format!("evt_first_{n}");
        let inserted =
            StripeEventMapper::record_first_time_in_tx(&mut tx, &id, "checkout.session.completed")
                .await
                .unwrap();
        assert!(inserted);
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn record_first_time_returns_false_on_replay() {
        let mut tx = crate::test_helpers::test_tx().await;
        let n: u64 = rand::random();
        let id = format!("evt_replay_{n}");
        // First-time insert: inserted.
        assert!(
            StripeEventMapper::record_first_time_in_tx(&mut tx, &id, "customer.subscription.updated")
                .await
                .unwrap()
        );
        // Replay: must short-circuit.
        let replayed =
            StripeEventMapper::record_first_time_in_tx(&mut tx, &id, "customer.subscription.updated")
                .await
                .unwrap();
        assert!(!replayed, "second insert with same id must return false");
        tx.rollback().await.unwrap();
    }
}
