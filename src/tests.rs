use crate::packet::{Packet, TYPE_AUTH_RESPONSE, TYPE_RESPONSE};

#[test]
fn packet_serialize() {
    let mut buf = vec!();
    Packet::new(0x12345678, TYPE_RESPONSE, "This is a test string.").send_internal(&mut buf).unwrap();

    assert_eq!(buf, vec![32, 0, 0, 0, 120, 86, 52, 18, 0, 0, 0, 0, 84, 104, 105, 115, 32, 105, 115, 32, 97, 32, 116, 101, 115, 116, 32, 115, 116, 114, 105, 110, 103, 46, 0, 0]);
}

#[test]
fn packet_deserialize() {
    let buf = vec![36u8, 0, 0, 0, 33, 67, 101, 119, 2, 0, 0, 0, 84, 104, 105, 115, 32, 105, 115, 32, 97, 32, 100, 105, 102, 102, 101, 114, 101, 110, 116, 32, 115, 116, 114, 105, 110, 103, 0, 0];
    let p = Packet::read(&mut &*buf).unwrap();
    assert_eq!(*p.get_id(), 0x77654321);
    assert_eq!(*p.get_packet_type(), TYPE_AUTH_RESPONSE);
    assert_eq!(*p.get_body(), "This is a different string".to_string());
}
