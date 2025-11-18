use {
    solana_time_utils::{
        AtomicInterval,
    },
    std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
        },
    },
};

const METRICS_REPORT_INTERVAL_MS: u64 = 10_000;

#[derive(Default)]
pub struct LedgerStorageStats {
    num_queries: AtomicUsize,
    last_report: AtomicInterval,
}

impl LedgerStorageStats {
    pub fn increment_num_queries(&self) {
        self.num_queries.fetch_add(1, Ordering::Relaxed);
        self.maybe_report();
    }

    pub fn maybe_report(&self) {
        if self.last_report.should_update(METRICS_REPORT_INTERVAL_MS) {
            // datapoint_debug!(
            //     "storage-bigtable-query",
            //     (
            //         "num_queries",
            //         self.num_queries.swap(0, Ordering::Relaxed) as i64,
            //         i64
            //     )
            // );
        }
    }
}