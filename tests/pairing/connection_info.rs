use lanqrus::ConnectionInfo;

#[test]
fn reject_unsupported_version() {
	let payload = r#"{"version":2,"addresses":["192.168.1.20:54321"]}"#;
	assert!(
		ConnectionInfo::from_qr_payload(payload).is_err()
	);
}

#[test]
fn rejects_malformed_payload() {
	assert!(
		ConnectionInfo::from_qr_payload("not-json")
			.is_err()
	);
}
