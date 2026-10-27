use super::*;

#[test]
fn networking_is_disabled_without_initialization() {
    assert!(!is_network_enabled());
    assert_eq!(require_network(), Err(NetworkDisabled));
}

#[test]
fn denial_does_not_disclose_requested_urls_or_credentials() {
    assert_eq!(
        require_network().unwrap_err().to_string(),
        "application networking is disabled in Warp Intranet"
    );
}
