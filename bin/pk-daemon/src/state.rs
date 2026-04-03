//! In-memory agent state with TTL-based auto-locking and zeroization.

use std::time::{Duration, Instant};

use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroize;

/// Time-to-live for an unlocked session.
pub const UNLOCK_TTL: Duration = Duration::from_mins(15);

/// Internal state of the passkey agent daemon.
#[derive(Debug)]
pub struct AgentState {
    age_private_key: Option<SecretString>,
    unlock_time: Option<Instant>,
}

impl AgentState {
    /// Create a new locked agent state.
    #[must_use]
    pub const fn new() -> Self {
        Self { age_private_key: None, unlock_time: None }
    }

    /// Unlock the agent with the provided age private key string.
    pub fn unlock(&mut self, key: String) {
        self.age_private_key = Some(SecretString::new(key.into()));
        self.unlock_time = Some(Instant::now());
    }

    /// Lock the agent, clearing the key from memory.
    pub fn lock(&mut self) {
        self.age_private_key = None;
        self.unlock_time = None;
    }

    /// Check whether the agent is currently unlocked and within TTL.
    #[must_use]
    pub fn is_unlocked(&self) -> bool {
        match (self.age_private_key.as_ref(), self.unlock_time) {
            (Some(_), Some(unlock_time)) => Instant::now().duration_since(unlock_time) < UNLOCK_TTL,
            _ => false,
        }
    }

    /// Return the remaining TTL duration, or `None` if locked or expired.
    #[must_use]
    pub fn ttl_remaining(&self) -> Option<Duration> {
        match self.unlock_time {
            Some(unlock_time) => {
                let elapsed = Instant::now().duration_since(unlock_time);
                (elapsed < UNLOCK_TTL).then(|| {
                    // Safety: checked_sub is called only when elapsed < UNLOCK_TTL,
                    // so the subtraction is guaranteed to succeed.
                    UNLOCK_TTL.checked_sub(elapsed).unwrap_or(Duration::ZERO)
                })
            }
            None => None,
        }
    }

    /// Return the age private key if unlocked and within TTL.
    #[must_use]
    pub fn get_key(&self) -> Option<String> {
        if self.is_unlocked() {
            self.age_private_key.as_ref().map(|s| s.expose_secret().to_string())
        } else {
            None
        }
    }
}

impl Default for AgentState {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AgentState {
    fn drop(&mut self) {
        if let Some(mut key) = self.age_private_key.take() {
            key.zeroize();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_is_locked() {
        let state = AgentState::new();
        assert!(!state.is_unlocked());
        assert!(state.ttl_remaining().is_none());
        assert!(state.get_key().is_none());
    }

    #[test]
    fn unlock_sets_key_and_ttl() {
        let mut state = AgentState::new();
        state.unlock("test-key".to_string());
        assert!(state.is_unlocked());
        assert!(state.ttl_remaining().is_some());
        assert_eq!(state.get_key(), Some("test-key".to_string()));
    }

    #[test]
    fn lock_clears_key_and_ttl() {
        let mut state = AgentState::new();
        state.unlock("test-key".to_string());
        state.lock();
        assert!(!state.is_unlocked());
        assert!(state.ttl_remaining().is_none());
        assert!(state.get_key().is_none());
    }

    #[test]
    fn ttl_remaining_decreases_over_time() {
        let mut state = AgentState::new();
        state.unlock("test-key".to_string());
        let first = state.ttl_remaining();
        std::thread::sleep(Duration::from_millis(10));
        let second = state.ttl_remaining();
        assert!(first.is_some());
        assert!(second.is_some());
        assert!(second <= first);
    }

    #[test]
    fn get_key_returns_none_when_locked() {
        let state = AgentState::new();
        assert!(state.get_key().is_none());
    }

    #[test]
    fn get_key_returns_some_when_unlocked() {
        let mut state = AgentState::new();
        state.unlock("my-secret-key".to_string());
        assert_eq!(state.get_key(), Some("my-secret-key".to_string()));
    }

    #[test]
    fn unlock_overwrites_previous_key() {
        let mut state = AgentState::new();
        state.unlock("old-key".to_string());
        state.unlock("new-key".to_string());
        assert_eq!(state.get_key(), Some("new-key".to_string()));
    }

    #[test]
    fn lock_is_idempotent() {
        let mut state = AgentState::new();
        state.lock();
        state.lock();
        assert!(!state.is_unlocked());
    }
}
