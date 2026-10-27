#!/usr/bin/env python3
"""Source regression guard; not a substitute for native build and packet-capture QA."""
from pathlib import Path
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
errors = []


def require(condition, message):
    if not condition:
        errors.append(message)


def source(path):
    return (ROOT / path).read_text()


def guarded(path, function, sink, guard="network_policy::require_network()"):
    text = source(path)
    start = text.index(function)
    end = text.index(sink, start)
    require(guard in text[start:end], f"{path}: {function} reaches {sink} without policy")


policy = source("crates/network_policy/src/lib.rs")
require("pub const fn is_network_enabled() -> bool {\n    false\n}" in policy,
        "network enablement must be immutable and default-deny")
require("pub const fn require_network() -> Result<(), NetworkDisabled> {\n    Err(NetworkDisabled)\n}" in policy,
        "all network operations must be rejected")
require(not re.search(r"std::env|env!|option_env!|Atomic|Mutex|cfg!|cfg\(feature", policy),
        "policy must not acquire environment, runtime or feature overrides")

guarded("crates/http_client/src/lib.rs", "async fn execute_inner", "self.wrapped.execute", "!network_policy::is_network_enabled()")
guarded("crates/http_client/src/lib.rs", "pub fn eventsource", ".eventsource()", "!network_policy::is_network_enabled()")
guarded("crates/websocket/src/native.rs", "pub async fn connect", "let request")
guarded("crates/websocket/src/wasm.rs", "pub async fn connect", "WsMeta::connect")
guarded("crates/websocket/src/proxy.rs", "pub async fn connect_via_proxy", "TcpStream::connect")
guarded("crates/asset_cache/src/lib.rs", "async fn fetch_file_to_memory", "reqwest::get")
guarded("crates/mcp/src/runtime.rs", "pub async fn spawn_server", "let mut is_authenticated_transport")
guarded("crates/mcp/src/oauth.rs", "pub async fn make_authenticated_client", "callback_mode.prepare()")
guarded("crates/mcp/src/oauth/loopback.rs", "pub async fn bind", "TcpListener::bind")
guarded("crates/local_control/src/client.rs", "pub fn send_request", "request_credential")
guarded("crates/http_server/src/lib.rs", "fn spawn_server", "let runtime")
guarded("crates/ai/src/grok_subscription/oauth.rs", "fn bind_callback_listener", "TcpListener::bind")
guarded("app/src/tracing/cloud_agent_auth.rs", "async fn send_bytes", "self.authorize_request")
guarded("app/src/crash_reporting/sentry_minidump.rs", "pub fn run_server", "sentry::init")
guarded("app/src/crash_reporting/mac.rs", "pub fn init_cocoa_sentry", "let endpoint", "!network_policy::is_network_enabled()")

# Inventory direct Rust transport constructors. New runtime files require a human network review.
# This is intentionally not a claim of whole-program call-graph coverage.
reviewed = {
    "app/src/ai/agent_sdk/driver/harness/acp/bridge.rs",
    "app/src/crash_reporting/mod.rs",
    "app/src/crash_reporting/sentry_minidump.rs",
    "app/src/local_control/mod.rs",
    "app/src/server/telemetry/mod.rs",
    "app/src/tracing/cloud_agent_auth.rs",
    "crates/ai/src/grok_subscription/oauth.rs",
    "crates/asset_cache/src/lib.rs",
    "crates/http_client/src/lib.rs",
    "crates/http_server/src/lib.rs",
    "crates/local_control/src/client.rs",
    "crates/mcp/src/oauth/loopback.rs",
    "crates/mcp/src/runtime.rs",
    "crates/mcp/src/sse_transport/reqwest_impl.rs",
    "crates/websocket/src/native.rs",
    "crates/websocket/src/proxy.rs",
    "crates/websocket/src/wasm.rs",
}
pattern = re.compile(r"reqwest::(?:get|Client::|blocking::Client::)|TcpStream::connect|TcpListener::bind|UdpSocket::|connect_async_with_tls_connector|sentry::init|WsMeta::connect")
for root in (ROOT / "app/src", ROOT / "crates"):
    for path in root.rglob("*.rs"):
        name = path.relative_to(ROOT).as_posix()
        if any(part in name for part in ("tests", "/examples/", "/integration_testing/", "test_support.rs", "crates/serve-wasm/")):
            continue
        if pattern.search(path.read_text()):
            require(name in reviewed, f"unreviewed direct network transport: {name}")

entry = source("app/src/bin/oss.rs")
require("WarpServerConfig::intranet()" in entry and "OzConfig::intranet()" in entry,
        "OSS entrypoint must not select vendor service endpoints")
for field in ("telemetry_config", "crash_reporting_config", "autoupdate_config", "mcp_static_config"):
    require(f"{field}: None" in entry, f"OSS entrypoint must disable {field}")
root = source("app/src/root_view.rs")
require("network_policy::is_network_enabled()" in root, "clean startup must bypass cloud onboarding")
features = source("crates/warp_features/src/lib.rs")
require("intranet::" in features, "feature overrides must pass through intranet policy")
require("network_policy::is_network_enabled()" in source("crates/warpui_core/src/telemetry/mod.rs"),
        "telemetry collection must be disabled, not merely uploads")
for path in (ROOT / ".github/workflows").glob("*.y*ml"):
    require("schedule:" not in path.read_text(), f"scheduled automation not allowed: {path.name}")
require((ROOT / "LICENSE-AGPL").is_file() and (ROOT / "LICENSE-MIT").is_file(), "preserve upstream licenses")
with (ROOT / "Cargo.lock").open("rb") as file:
    packages = tomllib.load(file)["package"]
require(any(p["name"] == "network_policy" for p in packages), "policy crate missing from lockfile")

if errors:
    print("Intranet source guard FAILED:", *errors, sep="\n- ", file=sys.stderr)
    sys.exit(1)
print(f"Intranet source guard passed; {len(reviewed)} direct transport files reviewed.")
print("Native macOS build, clean-profile startup and packet-capture acceptance are still required.")
