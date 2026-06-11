Feature: Daemon State Management
  As a user of passkey-agent
  I want the daemon to manage its locked/unlocked state with TTL
  So that my age key is only available in memory for a limited time

  Scenario: New daemon starts in locked state
    Given a new daemon instance
    When I query its state
    Then the state should be locked

  Scenario: Unlocking the daemon stores the key
    Given a locked daemon
    When I send an unlock request with a valid key
    Then the daemon should be unlocked
    And the key should be retrievable

  Scenario: Locking the daemon clears the key
    Given an unlocked daemon
    When I send a lock request
    Then the daemon should be locked
    And the key should not be retrievable

  Scenario: TTL expiry auto-locks the daemon
    Given an unlocked daemon with a short TTL
    When the TTL expires
    Then the daemon should be locked automatically
