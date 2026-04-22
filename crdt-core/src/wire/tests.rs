use super::*;
use crate::store::DeleteSet;
use crate::structs::Block;
use crate::types::{BlockId, ClientId, Clock};

fn bid(client: u64, clock: u64) -> BlockId {
    BlockId::new(ClientId::new(client), Clock::new(clock))
}

#[test]
fn wire_block_round_trips_through_block() {
    let original = Block::new(bid(1, 0), None, Some(bid(2, 5)), "hello".to_string());
    let wire = WireBlock::from(&original);
    let recovered = Block::from(wire.clone());

    assert_eq!(recovered.id, original.id);
    assert_eq!(recovered.origin_left, original.origin_left);
    assert_eq!(recovered.origin_right, original.origin_right);
    assert_eq!(recovered.content(), original.content());
    assert_eq!(recovered.len, original.len);
    assert!(!recovered.is_deleted);
}

#[test]
fn wire_block_with_both_origins_round_trips() {
    let wire = WireBlock {
        id: bid(3, 10),
        origin_left: Some(bid(1, 4)),
        origin_right: Some(bid(2, 7)),
        content: "x".to_string(),
    };
    let wire2 = wire.clone();
    let block = Block::from(wire);
    assert_eq!(WireBlock::from(&block), wire2);
}

#[test]
fn wire_block_bitcode_round_trip() {
    let wire = WireBlock {
        id: bid(7, 3),
        origin_left: Some(bid(1, 0)),
        origin_right: None,
        content: "hello".to_string(),
    };
    let bytes = bitcode::encode(&wire);
    let decoded: WireBlock = bitcode::decode(&bytes).expect("decode");
    assert_eq!(decoded, wire);
}

#[test]
fn encode_decode_insert_round_trips() {
    let msg = OpMessage::Insert(WireBlock {
        id: bid(1, 0),
        origin_left: None,
        origin_right: None,
        content: "hi".to_string(),
    });
    let frame = encode_op(&msg);
    assert_eq!(frame[0], OP_PREFIX);
    let decoded = decode_op(&frame).expect("decode");
    assert_eq!(decoded, msg);
}

#[test]
fn encode_decode_delete_round_trips() {
    let mut ds = DeleteSet::new();
    ds.add(bid(1, 0), 3);
    ds.add(bid(2, 5), 2);
    let msg = OpMessage::Delete(ds);
    let frame = encode_op(&msg);
    assert_eq!(frame[0], OP_PREFIX);
    let decoded = decode_op(&frame).expect("decode");
    assert_eq!(decoded, msg);
}

#[test]
fn decode_op_rejects_empty_frame() {
    assert!(matches!(decode_op(&[]), Err(WireError::EmptyFrame)));
}

#[test]
fn decode_op_rejects_snapshot_prefix() {
    let frame = vec![SNAPSHOT_PREFIX, 0x00];
    assert!(matches!(decode_op(&frame), Err(WireError::NotAnOp)));
}

#[test]
fn decode_op_rejects_unknown_prefix() {
    let frame = vec![0xFF, 0x00];
    assert!(matches!(
        decode_op(&frame),
        Err(WireError::UnknownPrefix(0xFF))
    ));
}

#[test]
fn decode_op_surfaces_bitcode_error_on_garbage_payload() {
    let frame = vec![OP_PREFIX, 0xFF, 0xFF, 0xFF, 0xFF];
    assert!(matches!(decode_op(&frame), Err(WireError::Decode(_))));
}

#[test]
fn wire_error_display_has_stable_text() {
    let e = WireError::EmptyFrame;
    assert!(!format!("{e}").is_empty());
}
