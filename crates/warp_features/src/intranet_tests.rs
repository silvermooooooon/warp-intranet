use crate::FeatureFlag;

#[test]
fn terminal_startup_does_not_require_an_account() {
    assert!(FeatureFlag::SkipFirebaseAnonymousUser.is_enabled());
    assert!(!FeatureFlag::ForceLogin.is_enabled());
    assert!(!FeatureFlag::AccountFirstOnboarding.is_enabled());
    assert!(!FeatureFlag::AgentOnboarding.is_enabled());
}

#[test]
fn vendor_services_are_unavailable() {
    assert!(!FeatureFlag::AgentMode.is_enabled());
    assert!(!FeatureFlag::CloudObjects.is_enabled());
    assert!(!FeatureFlag::McpOauth.is_enabled());
    assert!(!FeatureFlag::CreatingSharedSessions.is_enabled());
    assert!(!FeatureFlag::Autoupdate.is_enabled());
    assert!(!FeatureFlag::CrashReporting.is_enabled());
    assert!(!FeatureFlag::WithSandboxTelemetry.is_enabled());
    assert!(!FeatureFlag::SshRemoteServer.is_enabled());
}

#[cfg(feature = "test-util")]
#[test]
fn runtime_overrides_cannot_restore_vendor_services() {
    let _agent = FeatureFlag::AgentMode.override_enabled(true);
    let _login = FeatureFlag::ForceLogin.override_enabled(true);
    let _skip = FeatureFlag::SkipFirebaseAnonymousUser.override_enabled(false);

    assert!(!FeatureFlag::AgentMode.is_enabled());
    assert!(!FeatureFlag::ForceLogin.is_enabled());
    assert!(FeatureFlag::SkipFirebaseAnonymousUser.is_enabled());
}

#[cfg(feature = "test-util")]
#[test]
fn local_terminal_preferences_remain_configurable() {
    let enabled_override = FeatureFlag::NativeShellCompletions.override_enabled(true);
    assert!(FeatureFlag::NativeShellCompletions.is_enabled());
    drop(enabled_override);
    let _disabled = FeatureFlag::NativeShellCompletions.override_enabled(false);
    assert!(!FeatureFlag::NativeShellCompletions.is_enabled());
}
