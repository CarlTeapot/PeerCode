use super::Document;
use crate::structs::Block;
use crate::types::{BlockId, ClientId, Clock};

fn block_id(client: u64, clock: u64) -> BlockId {
    BlockId::new(ClientId::new(client), Clock::new(clock))
}

fn doc_with_single_block(content: &str) -> (Document, BlockId) {
    let client_id = ClientId::new(1);
    let id = BlockId::new(client_id, Clock::new(0));
    let mut doc = Document::new(client_id);
    doc.head = Some(id);
    doc.store
        .insert(Block::new(id, None, None, content.to_string()));
    (doc, id)
}

fn doc_with_two_blocks(left: &str, right: &str) -> (Document, BlockId, BlockId) {
    let client_id = ClientId::new(1);
    let left_id = BlockId::new(client_id, Clock::new(0));
    let right_id = left_id.at_offset(left.chars().count() as u64);
    let mut doc = Document::new(client_id);

    let mut left_block = Block::new(left_id, None, Some(right_id), left.to_string());
    left_block.set_right(Some(right_id));
    let mut right_block = Block::new(right_id, Some(left_id), None, right.to_string());
    right_block.set_left(Some(left_id));

    doc.head = Some(left_id);
    doc.store.insert(left_block);
    doc.store.insert(right_block);

    (doc, left_id, right_id)
}

#[test]
fn split_block_in_middle_updates_links_and_content() {
    let (mut doc, id) = doc_with_single_block("hello");

    doc.split_block(id, 2);

    let left = doc.store.get(&id).unwrap();
    let right_id = left.right().unwrap();
    let right = doc.store.get(&right_id).unwrap();

    assert_eq!(left.content(), "he");
    assert_eq!(right.content(), "llo");
    assert_eq!(right_id, id.at_offset(2));
    assert_eq!(left.right(), Some(right_id));
    assert_eq!(right.left(), Some(id));
    assert_eq!(right.origin_left, Some(id));
}

#[test]
fn split_block_at_zero_is_noop() {
    let (mut doc, id) = doc_with_single_block("hello");

    doc.split_block(id, 0);

    let block = doc.store.get(&id).unwrap();
    assert_eq!(block.content(), "hello");
    assert_eq!(block.right(), None);
}

#[test]
fn split_block_at_len_is_noop() {
    let (mut doc, id) = doc_with_single_block("hello");

    doc.split_block(id, 5);

    let block = doc.store.get(&id).unwrap();
    assert_eq!(block.content(), "hello");
    assert_eq!(block.right(), None);
}

#[test]
fn split_block_past_len_is_noop() {
    let (mut doc, id) = doc_with_single_block("hello");

    doc.split_block(id, 99);

    let block = doc.store.get(&id).unwrap();
    assert_eq!(block.content(), "hello");
    assert_eq!(block.right(), None);
}

#[test]
fn split_block_updates_existing_right_neighbor() {
    let (mut doc, left_id, right_id) = doc_with_two_blocks("abc", "def");

    doc.split_block(left_id, 1);

    let left = doc.store.get(&left_id).unwrap();
    let middle_id = left.right().unwrap();
    let middle = doc.store.get(&middle_id).unwrap();
    let right = doc.store.get(&right_id).unwrap();

    assert_eq!(left.content(), "a");
    assert_eq!(middle.content(), "bc");
    assert_eq!(right.content(), "def");
    assert_eq!(middle.left(), Some(left_id));
    assert_eq!(middle.right(), Some(right_id));
    assert_eq!(right.left(), Some(middle_id));
}

#[test]
fn split_deleted_block_keeps_both_halves_deleted() {
    let (mut doc, id) = doc_with_single_block("hello");
    doc.store.get_mut(&id).unwrap().is_deleted = true;

    doc.split_block(id, 2);

    let left = doc.store.get(&id).unwrap();
    let right_id = left.right().unwrap();
    let right = doc.store.get(&right_id).unwrap();

    assert!(left.is_deleted);
    assert!(right.is_deleted);
}

#[test]
fn get_block_and_offset_by_position_finds_first_block() {
    let (doc, left_id, _) = doc_with_two_blocks("abc", "def");

    let (found, offset) = doc.get_block_and_offset_by_position(2);

    assert_eq!(found, Some(left_id));
    assert_eq!(offset, 2);
}

#[test]
fn get_block_and_offset_by_position_finds_second_block() {
    let (doc, _, right_id) = doc_with_two_blocks("abc", "def");

    let (found, offset) = doc.get_block_and_offset_by_position(4);

    assert_eq!(found, Some(right_id));
    assert_eq!(offset, 1);
}

#[test]
fn get_block_and_offset_by_position_returns_none_past_end() {
    let (doc, _, _) = doc_with_two_blocks("abc", "def");

    let (found, offset) = doc.get_block_and_offset_by_position(7);

    assert_eq!(found, None);
    assert_eq!(offset, 1);
}

#[test]
fn get_block_and_offset_by_position_uses_character_offsets_for_unicode() {
    let client_id = ClientId::new(1);
    let left_id = block_id(1, 0);
    let right_id = block_id(1, 2);
    let mut doc = Document::new(client_id);

    let mut left = Block::new(left_id, None, Some(right_id), "a😀".to_string());
    left.set_right(Some(right_id));
    let mut right = Block::new(right_id, Some(left_id), None, "b".to_string());
    right.set_left(Some(left_id));

    doc.head = Some(left_id);
    doc.store.insert(left);
    doc.store.insert(right);

    let (found, offset) = doc.get_block_and_offset_by_position(2);

    assert_eq!(found, Some(right_id));
    assert_eq!(offset, 0);
}
