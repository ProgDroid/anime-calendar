//! Wrap database calls with duration + outcome metrics.

use crate::metrics::names::*;

/// Instrument a fallible async operation with a duration histogram and a
/// queries-total counter labelled by outcome.
pub async fn timed<F, T, E>(op: &'static str, fut: F) -> Result<T, E>
where
    F: std::future::Future<Output = Result<T, E>>,
{
    let start = std::time::Instant::now();
    let result = fut.await;
    let elapsed = start.elapsed().as_secs_f64();
    let outcome = if result.is_ok() { OUTCOME_OK } else { OUTCOME_ERR };

    metrics::histogram!(
        DB_QUERY_DURATION_SECONDS,
        LABEL_OP => op,
    )
    .record(elapsed);

    metrics::counter!(
        DB_QUERIES_TOTAL,
        LABEL_OP => op,
        LABEL_OUTCOME => outcome,
    )
    .increment(1);

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use metrics_util::debugging::Snapshotter;

    fn shared_snapshotter() -> Snapshotter {
        use metrics_util::debugging::DebuggingRecorder;
        use std::sync::OnceLock;
        static S: OnceLock<Snapshotter> = OnceLock::new();
        S.get_or_init(|| {
            let recorder = DebuggingRecorder::new();
            let snapshotter = recorder.snapshotter();
            let _ = recorder.install();
            snapshotter
        })
        .clone()
    }

    #[tokio::test]
    async fn timed_records_ok_outcome() {
        let snapshotter = shared_snapshotter();
        let _: Result<u32, ()> = timed("test.ok", async { Ok(42) }).await;
        let snapshot = snapshotter.snapshot().into_hashmap();
        let ok_count = snapshot
            .iter()
            .filter(|(k, _)| k.key().name() == DB_QUERIES_TOTAL)
            .filter(|(k, _)| {
                k.key().labels().any(|l| l.key() == LABEL_OUTCOME && l.value() == OUTCOME_OK)
            })
            .count();
        assert!(ok_count >= 1, "ok counter missing");
    }

    #[tokio::test]
    async fn timed_records_err_outcome() {
        let snapshotter = shared_snapshotter();
        let _: Result<(), &'static str> = timed("test.err", async { Err("boom") }).await;
        let snapshot = snapshotter.snapshot().into_hashmap();
        let err_count = snapshot
            .iter()
            .filter(|(k, _)| k.key().name() == DB_QUERIES_TOTAL)
            .filter(|(k, _)| {
                k.key().labels().any(|l| l.key() == LABEL_OUTCOME && l.value() == OUTCOME_ERR)
            })
            .count();
        assert!(err_count >= 1, "err counter missing");
    }
}
