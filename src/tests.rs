use crate::packet::{Packet, TYPE_AUTH_RESPONSE, TYPE_RESPONSE};

#[tokio::test]
async fn packet_serialize() {
	let buf = Packet::new(0x12345678, TYPE_RESPONSE, "This is a test string.".to_string())
		.create_packet_buffer()
		.unwrap();

	assert_eq!(
		buf,
		vec![
			32, 0, 0, 0, 120, 86, 52, 18, 0, 0, 0, 0, 84, 104, 105, 115, 32, 105, 115, 32, 97, 32, 116, 101, 115, 116,
			32, 115, 116, 114, 105, 110, 103, 46, 0, 0
		]
	);
}

#[tokio::test]
async fn packet_deserialize() {
	let buf = vec![
		33, 67, 101, 119, 2, 0, 0, 0, 84, 104, 105, 115, 32, 105, 115, 32, 97, 32, 100, 105, 102, 102, 101, 114, 101,
		110, 116, 32, 115, 116, 114, 105, 110, 103, 0, 0,
	];
	let p = Packet::decode_packet_buffer(36, &buf).unwrap();
	assert_eq!(p.get_id(), 0x77654321);
	assert_eq!(p.get_packet_type(), TYPE_AUTH_RESPONSE);
	assert_eq!(p.get_body(), "This is a different string");
}
/*
#[tokio::test]
async fn integration_test() {
	let mut c = Connection::open("localhost:25575", "test", Settings::default())
		.await
		.unwrap();
	c.exec("say Hi there!").await.unwrap();
	c.exec("say Hi there!").await.unwrap();
}
*/
