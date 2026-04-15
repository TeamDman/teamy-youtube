use std::time::Duration;
use tracing::info;

#[derive(Debug, Default, Eq, PartialEq)]
pub struct SyncProgress {
    items_total: usize,
    items_processed: usize,
    bytes_total: Option<u64>,
    bytes_processed: u64,
    last_written_file: Option<String>,
}

impl SyncProgress {
    #[must_use]
    pub fn new(items_total: usize) -> Self {
        Self {
            items_total,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn with_bytes_total(items_total: usize, bytes_total: u64) -> Self {
        Self {
            items_total,
            bytes_total: Some(bytes_total),
            ..Self::default()
        }
    }

    pub fn record_item(&mut self, processed_bytes: u64, last_written_file: Option<String>) {
        self.items_processed += 1;
        self.bytes_processed += processed_bytes;
        if let Some(last_written_file) = last_written_file {
            self.last_written_file = Some(last_written_file);
        }
    }

    #[must_use]
    pub fn items_total(&self) -> usize {
        self.items_total
    }

    #[must_use]
    pub fn items_processed(&self) -> usize {
        self.items_processed
    }

    pub fn emit_log(&self, operation: &'static str, elapsed: Duration) {
        let items_remaining = self.items_total.saturating_sub(self.items_processed);
        let bytes_per_second = bytes_per_second(self.bytes_processed, elapsed);
        let (bytes_total, bytes_total_is_estimate) = self.total_bytes();
        let bytes_remaining = bytes_total.saturating_sub(self.bytes_processed);
        let elapsed_seconds = elapsed.as_secs();
        let eta_seconds = estimate_eta_seconds(bytes_remaining, bytes_per_second);
        let eta_duration = Duration::from_secs(eta_seconds);

        info!(
            operation,
            items_total = self.items_total,
            items_processed = self.items_processed,
            items_remaining,
            bytes_total,
            bytes_total_human = %format_bytes(bytes_total),
            bytes_total_is_estimate,
            bytes_processed = self.bytes_processed,
            bytes_processed_human = %format_bytes(self.bytes_processed),
            bytes_remaining,
            bytes_remaining_human = %format_bytes(bytes_remaining),
            bytes_remaining_is_estimate = bytes_total_is_estimate,
            bytes_per_second,
            bytes_per_second_human = %format_bytes_per_second(bytes_per_second),
            elapsed_seconds,
            elapsed_humantime = %humantime::format_duration(elapsed),
            eta_seconds,
            eta_seconds_is_estimate = true,
            eta_humantime = %humantime::format_duration(eta_duration),
            last_written_file = self.last_written_file.as_deref().unwrap_or("none"),
            "sync progress"
        );
    }

    fn total_bytes(&self) -> (u64, bool) {
        match self.bytes_total {
            Some(bytes_total) => (bytes_total.max(self.bytes_processed), false),
            None => (estimate_total_bytes(self), true),
        }
    }
}

fn estimate_total_bytes(progress: &SyncProgress) -> u64 {
    if progress.items_processed == 0 {
        return 0;
    }

    let numerator = u128::from(progress.bytes_processed)
        * u128::try_from(progress.items_total).expect("usize should fit in u128");
    let denominator = u128::try_from(progress.items_processed).expect("usize should fit in u128");
    let estimated = (numerator + (denominator / 2)) / denominator;
    progress
        .bytes_processed
        .max(u64::try_from(estimated).unwrap_or(u64::MAX))
}

fn bytes_per_second(bytes_processed: u64, elapsed: Duration) -> u64 {
    let elapsed_nanos = elapsed.as_nanos();
    if elapsed_nanos == 0 {
        return 0;
    }

    let rate = (u128::from(bytes_processed) * 1_000_000_000_u128) / elapsed_nanos;
    u64::try_from(rate).unwrap_or(u64::MAX)
}

fn estimate_eta_seconds(bytes_remaining: u64, bytes_per_second: u64) -> u64 {
    if bytes_per_second == 0 {
        0
    } else {
        bytes_remaining.div_ceil(bytes_per_second)
    }
}

fn format_bytes_per_second(bytes_per_second: u64) -> String {
    format!("{}/s", format_bytes(bytes_per_second))
}

pub(crate) fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

    let mut divisor = 1_u128;
    let mut unit_index = 0_usize;
    while u128::from(bytes) >= divisor * 1024 && unit_index + 1 < UNITS.len() {
        divisor *= 1024;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        let scaled = ((u128::from(bytes) * 10) + (divisor / 2)) / divisor;
        let whole = scaled / 10;
        let fraction = scaled % 10;
        format!("{whole}.{fraction} {}", UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::format_bytes;

    #[test]
    fn formats_human_bytes() {
        assert_eq!(format_bytes(999), "999 B");
        assert_eq!(format_bytes(2_048), "2.0 KiB");
    }
}
