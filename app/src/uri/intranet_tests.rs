use super::*;

#[test]
fn intranet_rejects_service_callback_hosts() {
    for host in [
        "auth",
        "team",
        "shared_session",
        "conversation",
        "drive",
        "home",
        "mcp",
        "codex",
        "linear",
        CHATGPT_LINK_URI_HOST,
    ] {
        assert!(
            UriHost::from_str(host).is_err(),
            "accepted service host {host}"
        );
    }
}

#[test]
fn intranet_retains_local_uri_hosts() {
    for host in ["action", "launch", "settings", "session"] {
        assert!(
            UriHost::from_str(host).is_ok(),
            "rejected local host {host}"
        );
    }
}

#[test]
fn intranet_rejects_cloud_action_uris() {
    for path in [
        "cloud_agent_setup",
        "new_cloud_agent_conversation",
        "new_agent_conversation",
        "create_environment",
        "focus_cloud_mode",
        "auto_handoff_to_cloud",
        "auto-handoff-to-cloud",
    ] {
        let url = Url::parse(&format!("warp://action/{path}")).unwrap();
        assert!(
            Action::parse(&url).is_err(),
            "accepted service action {path}"
        );
    }
}

#[test]
fn intranet_retains_local_terminal_actions() {
    for path in ["new_tab", "new_window", "docker/open_subshell", "open-repo"] {
        let url = Url::parse(&format!("warp://action/{path}")).unwrap();
        assert!(Action::parse(&url).is_ok(), "rejected local action {path}");
    }
}

#[test]
fn intranet_rejects_web_intents() {
    let url = Url::parse("https://app.warp.dev/app").unwrap();
    assert!(web_intent_parser::WebIntent::try_from_url(&url).is_err());
}
