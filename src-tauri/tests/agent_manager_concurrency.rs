//! Integration-level smoke test for AgentManager concurrency.
//!
//! The core double-checked-lock concurrency tests live in
//! `src/infrastructure/agent/manager.rs` (unit tests) where `pub(crate)` internals
//! are accessible. This file verifies the public API surface doesn't deadlock
//! under rapid sequential calls.

use jarvis_lib::infrastructure::agent::AGENT_MANAGER;
use std::time::Duration;

#[tokio::test]
async fn restart_does_not_deadlock() {
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        let (r1, r2) = tokio::join!(
            tokio::spawn(async { AGENT_MANAGER.restart().await }),
            tokio::spawn(async { AGENT_MANAGER.restart().await }),
        );
        r1.expect("restart task 1 panicked");
        r2.expect("restart task 2 panicked");
    })
    .await;
    assert!(result.is_ok(), "concurrent restart deadlocked or timed out");
}
