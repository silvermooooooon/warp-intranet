use super::*;

#[test]
fn remote_assets_are_rejected_before_dns_or_transport() {
    let error = futures::executor::block_on(fetch_file_to_memory(
        Url::parse("https://fonts.example.invalid/font.ttf").unwrap(),
    ))
    .unwrap_err();
    assert!(
        error
            .downcast_ref::<network_policy::NetworkDisabled>()
            .is_some()
    );
}
