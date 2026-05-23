//! Typed GraphQL queries against Linear's schema.
//!
//! Populated in C1.5 via `cynic` codegen against the vendored schema
//! at `engine/houston-linear/schema/linear.graphql`. Each query module
//! corresponds to a Linear resource family:
//!
//! - `viewer` — current AppUser identity (for org-binding bootstrap).
//! - `teams` — workspace's teams (required for routing policy editor).
//! - `workflow_states` — per-team typed state catalog.
//! - `issues` — paginated list with `updatedAt > checkpoint` filter
//!   for polling reconciliation.
//! - `projects`, `initiatives`, `cycles` — capability-gated.
//!
//! Every paginated query uses explicit `first: N` to bound Linear's
//! complexity-points consumption. Default page size: 50. The
//! rate-limit budgeter ([`crate::rate_limit`]) tracks the per-query
//! cost and refuses to dispatch when the rolling window is exhausted.
