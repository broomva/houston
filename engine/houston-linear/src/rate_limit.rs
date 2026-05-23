//! Linear rate-limit budgeter.
//!
//! Linear's quota is [`crate::RATE_LIMIT_POINTS_PER_HOUR`] complexity
//! points per OAuth app, with per-query caps. The engine tracks a
//! rolling token bucket and refuses to dispatch queries that would
//! exceed the budget.
//!
//! ## Why complexity-aware, not request-count
//!
//! Linear charges per GraphQL **complexity** (a function of field
//! depth × child connections × `first: N` page sizes), not per HTTP
//! request. A single deep `issues(first: 50) { children(first: 50) }`
//! query can cost more than 100 shallow ones. Counting requests
//! would let a deep query exhaust the budget invisibly.
//!
//! ## Priority lanes
//!
//! When the rolling window is near exhaustion, the budgeter
//! prioritizes:
//! 1. AgentSessionEvent egress (5s budget is HARD).
//! 2. Webhook-triggered mutations (state writeback, comments).
//! 3. User-initiated reads (UI fetches on demand).
//! 4. Polling reconciles (the lowest priority — they have a webhook
//!    backstop).
//!
//! Reconciles back off exponentially when starved; the webhook stream
//! is the primary fresh-data path.
//!
//! ## Bucket math
//!
//! Refill rate = [`crate::RATE_LIMIT_POINTS_PER_HOUR`] / 3600 points
//! per second = ~833 points/sec. Bucket capacity = full hourly quota.
//! Each query estimates its cost (cynic-codegen surfaces the
//! complexity-points cost from the schema) and consumes that many
//! tokens before dispatch.
//!
//! Populated in C2.

/// Refill rate in points per second.
pub const REFILL_POINTS_PER_SEC: u32 = crate::RATE_LIMIT_POINTS_PER_HOUR / 3_600;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refill_rate_is_833_points_per_second() {
        // 3_000_000 / 3600 ≈ 833 (integer division)
        assert_eq!(REFILL_POINTS_PER_SEC, 833);
    }
}
