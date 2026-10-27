use super::*;

#[test]
fn websocket_is_rejected_before_a_loopback_connection() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("ws://{}/", listener.local_addr().unwrap());

    let error =
        futures::executor::block_on(WebSocket::connect(url.as_str(), std::iter::empty::<&str>()))
            .err()
            .unwrap();

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

#[test]
fn malformed_websocket_url_does_not_bypass_the_denial() {
    let error =
        futures::executor::block_on(WebSocket::connect("not a url", std::iter::empty::<&str>()))
            .err()
            .unwrap();
    assert!(
        error
            .downcast_ref::<network_policy::NetworkDisabled>()
            .is_some()
    );
}
