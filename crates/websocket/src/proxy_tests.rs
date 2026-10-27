use std::sync::Mutex;

use std::net::TcpListener;

use super::*;

/// Guard that ensures proxy-related env vars are cleaned up after each test.
/// Tests that manipulate env vars must hold this lock to avoid races.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn clear_proxy_env() {
    for var in [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
        "NO_PROXY",
        "no_proxy",
    ] {
        unsafe {
            env::remove_var(var);
        }
    }
}

fn wss_uri(host: &str) -> http::Uri {
    format!("wss://{host}").parse().unwrap()
}

fn ws_uri(host: &str) -> http::Uri {
    format!("ws://{host}").parse().unwrap()
}

fn resolved_proxy_tls(host: &str) -> Option<ProxyInfo> {
    resolve_proxy(&wss_uri(host)).expect("proxy resolution should succeed")
}

fn resolved_proxy_plain(host: &str) -> Option<ProxyInfo> {
    resolve_proxy(&ws_uri(host)).expect("proxy resolution should succeed")
}

// -- resolve_proxy tests --

#[test]
fn resolve_proxy_returns_none_when_no_env_vars_set() {
    let _lock = ENV_LOCK.lock();
    clear_proxy_env();
    assert!(resolved_proxy_tls("example.com").is_none());
    assert!(resolved_proxy_plain("example.com").is_none());
}

#[test]
fn resolve_proxy_reads_https_proxy_for_tls() {
    let _lock = ENV_LOCK.lock();
    clear_proxy_env();
    unsafe {
        env::set_var("HTTPS_PROXY", "http://proxy.corp:3128");
    }

    let info = resolved_proxy_tls("example.com").expect("should resolve");
    assert_eq!(info.host, "proxy.corp");
    assert_eq!(info.port, 3128);
    assert!(info.basic_auth.is_none());

    // Non-TLS should not use HTTPS_PROXY.
    assert!(resolved_proxy_plain("example.com").is_none());
    clear_proxy_env();
}

#[test]
fn resolve_proxy_reads_http_proxy_for_non_tls() {
    let _lock = ENV_LOCK.lock();
    clear_proxy_env();
    unsafe {
        env::set_var("HTTP_PROXY", "http://proxy.corp:8080");
    }

    let info = resolved_proxy_plain("example.com").expect("should resolve");
    assert_eq!(info.host, "proxy.corp");
    assert_eq!(info.port, 8080);

    // TLS should not use HTTP_PROXY.
    assert!(resolved_proxy_tls("example.com").is_none());
    clear_proxy_env();
}

#[test]
fn resolve_proxy_falls_back_to_all_proxy() {
    let _lock = ENV_LOCK.lock();
    clear_proxy_env();
    unsafe {
        env::set_var("ALL_PROXY", "http://all-proxy.corp:9999");
    }

    let tls_info = resolved_proxy_tls("example.com").expect("TLS should fall back to ALL_PROXY");
    assert_eq!(tls_info.host, "all-proxy.corp");
    assert_eq!(tls_info.port, 9999);

    let plain_info =
        resolved_proxy_plain("example.com").expect("plain should fall back to ALL_PROXY");
    assert_eq!(plain_info.host, "all-proxy.corp");
    assert_eq!(plain_info.port, 9999);
    clear_proxy_env();
}

#[test]
fn resolve_proxy_prefers_specific_over_all_proxy() {
    let _lock = ENV_LOCK.lock();
    clear_proxy_env();
    unsafe {
        env::set_var("HTTPS_PROXY", "http://specific:1111");
    }
    unsafe {
        env::set_var("ALL_PROXY", "http://fallback:2222");
    }

    let info = resolved_proxy_tls("example.com").expect("should resolve");
    assert_eq!(info.host, "specific");
    assert_eq!(info.port, 1111);
    clear_proxy_env();
}

#[test]
fn resolve_proxy_reads_lowercase_env_vars() {
    let _lock = ENV_LOCK.lock();
    clear_proxy_env();
    unsafe {
        env::set_var("https_proxy", "http://lower.corp:4444");
    }

    let info = resolved_proxy_tls("example.com").expect("should resolve from lowercase");
    assert_eq!(info.host, "lower.corp");
    assert_eq!(info.port, 4444);
    clear_proxy_env();
}

#[test]
fn resolve_proxy_returns_error_for_malformed_proxy_env() {
    let _lock = ENV_LOCK.lock();
    clear_proxy_env();
    unsafe {
        env::set_var("HTTPS_PROXY", "://broken");
    }

    let err = resolve_proxy(&wss_uri("example.com")).expect_err("malformed proxy env should fail");
    let err_msg = format!("{err:#}");
    assert!(err_msg.contains("Invalid proxy URL configured in HTTPS_PROXY"));
    assert!(err_msg.contains("failed to parse proxy URL"));
    clear_proxy_env();
}

#[test]
fn resolve_proxy_rejects_https_proxy_urls() {
    let _lock = ENV_LOCK.lock();
    clear_proxy_env();
    unsafe {
        env::set_var("HTTPS_PROXY", "https://proxy.corp:443");
    }

    let err = resolve_proxy(&wss_uri("example.com")).expect_err("https proxy URLs should fail");
    let err_msg = format!("{err:#}");
    assert!(err_msg.contains("Invalid proxy URL configured in HTTPS_PROXY"));
    assert!(err_msg.contains("HTTPS proxy URLs are not supported"));
    clear_proxy_env();
}

// -- NO_PROXY tests --

#[test]
fn no_proxy_exact_match() {
    let _lock = ENV_LOCK.lock();
    clear_proxy_env();
    unsafe {
        env::set_var("HTTPS_PROXY", "http://proxy:3128");
    }
    unsafe {
        env::set_var("NO_PROXY", "example.com");
    }

    assert!(resolved_proxy_tls("example.com").is_none());
    assert!(resolved_proxy_tls("other.com").is_some());
    clear_proxy_env();
}

#[test]
fn no_proxy_wildcard() {
    let _lock = ENV_LOCK.lock();
    clear_proxy_env();
    unsafe {
        env::set_var("HTTPS_PROXY", "http://proxy:3128");
    }
    unsafe {
        env::set_var("NO_PROXY", "*");
    }

    assert!(resolved_proxy_tls("anything.com").is_none());
    clear_proxy_env();
}

#[test]
fn no_proxy_suffix_with_dot() {
    let _lock = ENV_LOCK.lock();
    clear_proxy_env();
    unsafe {
        env::set_var("HTTPS_PROXY", "http://proxy:3128");
    }
    unsafe {
        env::set_var("NO_PROXY", ".warp.dev");
    }

    assert!(resolved_proxy_tls("sessions.app.warp.dev").is_none());

    assert!(resolved_proxy_tls("warp.dev").is_some()); // Exact "warp.dev" != ".warp.dev"
    assert!(resolved_proxy_tls("other.com").is_some());
    clear_proxy_env();
}

#[test]
fn no_proxy_suffix_without_dot() {
    let _lock = ENV_LOCK.lock();
    clear_proxy_env();
    unsafe {
        env::set_var("HTTPS_PROXY", "http://proxy:3128");
    }
    unsafe {
        env::set_var("NO_PROXY", "warp.dev");
    }

    // "sessions.app.warp.dev" ends with ".warp.dev" → matches
    assert!(resolved_proxy_tls("sessions.app.warp.dev").is_none());
    // Exact match too
    assert!(resolved_proxy_tls("warp.dev").is_none());
    assert!(resolved_proxy_tls("notwarp.dev").is_some());
    clear_proxy_env();
}

#[test]
fn no_proxy_comma_separated() {
    let _lock = ENV_LOCK.lock();
    clear_proxy_env();
    unsafe {
        env::set_var("HTTPS_PROXY", "http://proxy:3128");
    }
    unsafe {
        env::set_var("NO_PROXY", "localhost, 127.0.0.1, .internal.corp");
    }

    assert!(resolved_proxy_tls("localhost").is_none());
    assert!(resolved_proxy_tls("127.0.0.1").is_none());
    assert!(resolved_proxy_tls("foo.internal.corp").is_none());
    assert!(resolved_proxy_tls("external.com").is_some());
    clear_proxy_env();
}

#[test]
fn no_proxy_case_insensitive() {
    let _lock = ENV_LOCK.lock();
    clear_proxy_env();
    unsafe {
        env::set_var("HTTPS_PROXY", "http://proxy:3128");
    }
    unsafe {
        env::set_var("NO_PROXY", "Example.COM");
    }

    assert!(resolved_proxy_tls("example.com").is_none());
    assert!(resolved_proxy_tls("EXAMPLE.COM").is_none());
    clear_proxy_env();
}

// -- parse_proxy_url tests --

#[test]
fn parse_proxy_url_with_scheme() {
    let info = parse_proxy_url("http://proxy.corp:3128").expect("should parse");
    assert_eq!(info.host, "proxy.corp");
    assert_eq!(info.port, 3128);
    assert!(info.basic_auth.is_none());
}

#[test]
fn parse_proxy_url_without_scheme() {
    let info = parse_proxy_url("proxy.corp:8080").expect("should parse");
    assert_eq!(info.host, "proxy.corp");
    assert_eq!(info.port, 8080);
}

#[test]
fn parse_proxy_url_default_port() {
    let info = parse_proxy_url("http://proxy.corp").expect("should parse");
    assert_eq!(info.port, 80);
}

#[test]
fn parse_proxy_url_explicit_default_port() {
    // Explicit :80 should resolve to 80, not be swallowed by the URL parser.
    let info = parse_proxy_url("http://proxy.corp:80").expect("should parse");
    assert_eq!(info.port, 80);
}

#[test]
fn parse_proxy_url_with_credentials() {
    let info = parse_proxy_url("http://user:pass@proxy.corp:3128").expect("should parse");
    assert_eq!(info.host, "proxy.corp");
    assert_eq!(info.port, 3128);
    let decoded = String::from_utf8(
        base64::engine::general_purpose::STANDARD
            .decode(info.basic_auth.as_ref().unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(decoded, "user:pass");
}

#[test]
fn parse_proxy_url_decodes_percent_encoded_credentials() {
    let info = parse_proxy_url("http://user%40name:p%3Ass@proxy.corp:3128").expect("should parse");
    let decoded = String::from_utf8(
        base64::engine::general_purpose::STANDARD
            .decode(info.basic_auth.as_ref().expect("basic auth should exist"))
            .expect("basic auth should be valid base64"),
    )
    .expect("decoded basic auth should be valid UTF-8");
    assert_eq!(decoded, "user@name:p:ss");
}

// Proxy negotiation is intentionally unavailable in the intranet distribution.

#[tokio::test]
async fn connect_via_proxy_is_rejected_before_opening_a_socket() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let proxy_info = ProxyInfo {
        host: "127.0.0.1".to_string(),
        port: listener.local_addr().unwrap().port(),
        basic_auth: None,
    };
    let target_uri: http::Uri = "wss://target.example.com:443".parse().unwrap();

    let error = connect_via_proxy(&proxy_info, &target_uri)
        .await
        .unwrap_err();

    assert!(
        error
            .downcast_ref::<network_policy::NetworkDisabled>()
            .is_some()
    );
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[tokio::test]
async fn proxy_credentials_cannot_restore_a_connection_or_leak_in_errors() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let proxy_info = ProxyInfo {
        host: "127.0.0.1".to_string(),
        port: listener.local_addr().unwrap().port(),
        basic_auth: Some(BASE64.encode("user:secret")),
    };
    let target_uri: http::Uri = "wss://host.example.com:8443".parse().unwrap();

    let error = connect_via_proxy(&proxy_info, &target_uri)
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "application networking is disabled in Warp Intranet"
    );
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[tokio::test]
async fn policy_rejects_proxy_requests_before_target_validation() {
    let proxy_info = ProxyInfo {
        host: "proxy.example.invalid".to_string(),
        port: 8080,
        basic_auth: None,
    };
    let target_uri: http::Uri = "/no-target-host".parse().unwrap();

    let error = connect_via_proxy(&proxy_info, &target_uri)
        .await
        .unwrap_err();

    assert!(
        error
            .downcast_ref::<network_policy::NetworkDisabled>()
            .is_some()
    );
}
