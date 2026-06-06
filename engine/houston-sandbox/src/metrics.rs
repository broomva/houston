//! Latency aggregation for the benchmark harness. Collects per-phase
//! samples and reduces them to the percentiles that decide the architecture
//! (p50/p95/p99 of each lifecycle phase).

use serde::Serialize;

/// Raw millisecond samples for one lifecycle phase.
#[derive(Debug, Default, Clone)]
pub struct PhaseTimings {
    samples_ms: Vec<f64>,
}

impl PhaseTimings {
    /// Record one sample.
    pub fn record(&mut self, ms: f64) {
        self.samples_ms.push(ms);
    }

    /// Number of samples.
    pub fn count(&self) -> usize {
        self.samples_ms.len()
    }

    /// Nearest-rank percentile (`p` in 0..=100). `None` if empty.
    pub fn percentile(&self, p: f64) -> Option<f64> {
        if self.samples_ms.is_empty() {
            return None;
        }
        let mut sorted = self.samples_ms.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).expect("no NaN in latency samples"));
        let rank = (p / 100.0 * (sorted.len() as f64 - 1.0)).round() as usize;
        Some(sorted[rank.min(sorted.len() - 1)])
    }

    /// Arithmetic mean. `None` if empty.
    pub fn mean(&self) -> Option<f64> {
        if self.samples_ms.is_empty() {
            return None;
        }
        Some(self.samples_ms.iter().sum::<f64>() / self.samples_ms.len() as f64)
    }

    /// Reduce to a serializable summary. `None` if no samples were recorded.
    /// Sorts the samples once and indexes all percentiles from that copy.
    pub fn summarize(&self, phase: &str) -> Option<PhaseSummary> {
        if self.samples_ms.is_empty() {
            return None;
        }
        let mut sorted = self.samples_ms.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).expect("no NaN in latency samples"));
        let at = |p: f64| {
            let rank = (p / 100.0 * (sorted.len() as f64 - 1.0)).round() as usize;
            sorted[rank.min(sorted.len() - 1)]
        };
        let mean = self.samples_ms.iter().sum::<f64>() / self.samples_ms.len() as f64;
        Some(PhaseSummary {
            phase: phase.to_string(),
            count: sorted.len(),
            min_ms: at(0.0),
            p50_ms: at(50.0),
            p95_ms: at(95.0),
            p99_ms: at(99.0),
            max_ms: at(100.0),
            mean_ms: mean,
        })
    }
}

/// Percentile summary for one phase — the unit the report table is built from.
#[derive(Debug, Clone, Serialize)]
pub struct PhaseSummary {
    pub phase: String,
    pub count: usize,
    pub min_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
    pub mean_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_on_known_set() {
        let mut t = PhaseTimings::default();
        for v in 1..=100 {
            t.record(v as f64);
        }
        assert_eq!(t.percentile(0.0), Some(1.0));
        assert_eq!(t.percentile(100.0), Some(100.0));
        // nearest-rank p50 of 1..=100 → index round(0.5*99)=50 → value 51
        assert_eq!(t.percentile(50.0), Some(51.0));
        assert_eq!(t.mean(), Some(50.5));
    }

    #[test]
    fn empty_yields_none() {
        let t = PhaseTimings::default();
        assert_eq!(t.percentile(50.0), None);
        assert!(t.summarize("x").is_none());
    }
}
