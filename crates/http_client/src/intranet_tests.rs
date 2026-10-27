use super::*;

#[test]
fn http_is_rejected_before_request_hooks_or_network_io() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut client = Client::new_external();
    client.set_before_request_fn(Box::new(|_, _| panic!("blocked request reached hook")));
    let url = format!("http://{}/must-not-connect", listener.local_addr().unwrap());

    let error = futures::executor::block_on(client.get(url).send())
        .err()
        .unwrap();

    assert!(error.is_builder());
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn public_hostname_is_rejected_without_a_tokio_runtime_or_dns() {
    let client = Client::new_external();
    let error = futures::executor::block_on(client.get("https://app.warp.dev/").send())
        .err()
        .unwrap();
    assert!(error.is_builder());
}

#[test]
fn server_sent_events_fail_once_without_starting_a_transport() {
    let client = Client::new_external();
    let mut stream = client.get("https://app.warp.dev/").eventsource();
    assert!(futures::executor::block_on(stream.next()).unwrap().is_err());
    assert!(futures::executor::block_on(stream.next()).is_none());
}

#[test]
fn oauth_uses_the_same_blocked_transport() {
    use oauth2::AsyncHttpClient;
    let client = Client::new_external();
    let request = http::Request::builder()
        .uri("https://accounts.example.invalid/token")
        .body(Vec::new())
        .unwrap();
    assert!(futures::executor::block_on(client.call(request)).is_err());
}

#[test]
fn explicit_proxy_and_redirect_configuration_cannot_restore_egress() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let client = Client::from_client_builder(
        reqwest::Client::builder()
            .proxy(reqwest::Proxy::all(&url).unwrap())
            .redirect(reqwest::redirect::Policy::limited(10)),
    )
    .unwrap();

    let error = futures::executor::block_on(client.get("https://192.0.2.1/").send())
        .err()
        .unwrap();

    assert!(error.is_builder());
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn ipv6_loopback_is_not_an_allowed_exception() {
    let client = Client::new_external();
    let error = futures::executor::block_on(client.get("http://[::1]:8080/").send())
        .err()
        .unwrap();
    assert!(error.is_builder());
}
