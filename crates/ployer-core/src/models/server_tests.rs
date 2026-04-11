use super::*;

#[test]
fn server_status_round_trips() {
    for (variant, s) in [
        (ServerStatus::Online, "online"),
        (ServerStatus::Offline, "offline"),
        (ServerStatus::Unknown, "unknown"),
    ] {
        assert_eq!(variant.as_str(), s);
        assert_eq!(ServerStatus::from_str(s), variant);
    }
}

#[test]
fn server_status_unknown_defaults_to_unknown() {
    assert_eq!(ServerStatus::from_str("unreachable"), ServerStatus::Unknown);
}
