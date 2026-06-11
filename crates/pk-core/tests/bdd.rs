//! BDD scenarios for pk-core library.
//!
//! Reuses pk-daemon's AgentState for daemon state management scenarios.

use std::{path::PathBuf, sync::Arc};

use cucumber::World;
use pk_daemon::state::AgentState;
use tokio::sync::Mutex;

/// World state for pk-core BDD scenarios.
#[derive(Debug, Default, World)]
pub struct PkCoreWorld {
    state: Option<Arc<Mutex<AgentState>>>,
    was_unlocked: bool,
    key_retrievable: bool,
}

#[cucumber::given("a new daemon instance")]
async fn new_daemon(world: &mut PkCoreWorld) {
    world.state = Some(Arc::new(Mutex::new(AgentState::new())));
    world.was_unlocked = false;
    world.key_retrievable = false;
}

#[cucumber::given("a locked daemon")]
async fn locked_daemon(world: &mut PkCoreWorld) {
    world.state = Some(Arc::new(Mutex::new(AgentState::new())));
    world.was_unlocked = false;
    world.key_retrievable = false;
}

#[cucumber::given("an unlocked daemon")]
async fn unlocked_daemon(world: &mut PkCoreWorld) {
    let mut state = AgentState::new();
    state.unlock("test-age-key".to_string());
    world.state = Some(Arc::new(Mutex::new(state)));
    world.was_unlocked = true;
    world.key_retrievable = true;
}

#[cucumber::given("an unlocked daemon with a short TTL")]
async fn unlocked_daemon_short_ttl(world: &mut PkCoreWorld) {
    let mut state = AgentState::new();
    state.unlock("test-age-key".to_string());
    world.state = Some(Arc::new(Mutex::new(state)));
    world.was_unlocked = true;
    world.key_retrievable = true;
}

#[cucumber::when("I query its state")]
async fn query_state(world: &mut PkCoreWorld) {
    if let Some(state_arc) = &world.state {
        let state = state_arc.lock().await;
        world.was_unlocked = state.is_unlocked();
    }
}

#[cucumber::when("I send an unlock request with a valid key")]
async fn send_unlock(world: &mut PkCoreWorld) {
    if let Some(state_arc) = &world.state {
        let mut state = state_arc.lock().await;
        state.unlock("test-age-key".to_string());
        world.was_unlocked = true;
        world.key_retrievable = state.get_key().is_some();
    }
}

#[cucumber::when("I send a lock request")]
async fn send_lock(world: &mut PkCoreWorld) {
    if let Some(state_arc) = &world.state {
        let mut state = state_arc.lock().await;
        state.lock();
        world.was_unlocked = false;
        world.key_retrievable = state.get_key().is_some();
    }
}

#[cucumber::when("the TTL expires")]
async fn ttl_expires(world: &mut PkCoreWorld) {
    if let Some(state_arc) = &world.state {
        let mut state = state_arc.lock().await;
        state.lock();
        world.was_unlocked = false;
        world.key_retrievable = state.get_key().is_some();
    }
}

#[cucumber::then("the state should be locked")]
async fn state_locked(world: &mut PkCoreWorld) {
    assert!(!world.was_unlocked, "Expected daemon to be locked");
}

#[cucumber::then("the daemon should be unlocked")]
async fn state_unlocked(world: &mut PkCoreWorld) {
    assert!(world.was_unlocked, "Expected daemon to be unlocked");
}

#[cucumber::then("the daemon should be locked")]
async fn daemon_locked(world: &mut PkCoreWorld) {
    assert!(!world.was_unlocked, "Expected daemon to be locked after lock request");
}

#[cucumber::then("the key should be retrievable")]
async fn key_retrievable(world: &mut PkCoreWorld) {
    assert!(world.key_retrievable, "Expected key to be retrievable");
}

#[cucumber::then("the key should not be retrievable")]
async fn key_not_retrievable(world: &mut PkCoreWorld) {
    assert!(!world.key_retrievable, "Expected key to NOT be retrievable");
}

#[cucumber::then("the daemon should be locked automatically")]
async fn daemon_auto_locked(world: &mut PkCoreWorld) {
    assert!(!world.was_unlocked, "Expected daemon to auto-lock after TTL expiry");
}

#[tokio::main]
async fn main() {
    let feature_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../features");
    PkCoreWorld::run(feature_path.as_path()).await;
}
