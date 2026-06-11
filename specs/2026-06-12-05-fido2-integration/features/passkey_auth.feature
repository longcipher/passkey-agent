Feature: Passkey Authentication
  As a user of passkey-agent
  I want to unlock my age key using my biometric passkey
  So that I can use passwordless authentication

  @fido2
  Scenario: Generate passkey credential during init
    Given the platform authenticator is available
    When I run pk init
    Then a passkey credential should be created
    And the credential ID should be stored in config
    And the KEK should be derived from hmac-secret

  @fido2
  Scenario: Authenticate with passkey to unlock
    Given a configured passkey credential
    And the daemon is locked
    When I run a sops command
    Then the platform authenticator should prompt for biometric
    And the KEK should be derived from hmac-secret
    And the DEK should be decrypted
    And the age key should be unlocked

  @fido2 @error
  Scenario: Passkey authentication cancelled by user
    Given a configured passkey credential
    And the daemon is locked
    When I run a sops command
    And the user cancels the biometric prompt
    Then the passkey route should fail
    And the password route should be attempted as fallback

  @fido2 @error
  Scenario: Passkey unavailable falls back to password
    Given no platform authenticator is available
    And a configured password route
    And the daemon is locked
    When I run a sops command
    Then the passkey route should be skipped
    And the password prompt should appear
