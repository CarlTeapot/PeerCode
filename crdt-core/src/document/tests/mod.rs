use super::Document;
use crate::store::DeleteSet;
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

fn visible_text(doc: &Document) -> String {
    let mut out = String::new();
    let mut cur = doc.head.and_then(|id| doc.store.get(&id));
    while let Some(block) = cur {
        if !block.is_deleted {
            out.push_str(block.content());
        }
        cur = block.right().and_then(|id| doc.store.get(&id));
    }
    out
}

fn bid(c: u64, clock: u64) -> BlockId {
    BlockId::new(ClientId::new(c), Clock::new(clock))
}

#[test]
fn insert_into_empty_document() {
    let mut doc = Document::new(ClientId::new(1));
    doc.local_insert(0, "Test").unwrap();

    assert_eq!(doc.get_text(), "Test");
    assert_eq!(doc.state_vector.get(&ClientId::new(1)), 4);
}

#[test]
fn insert_append_prepend_and_middle() {
    let mut doc = Document::new(ClientId::new(1));

    doc.local_insert(0, "Vaime").unwrap();
    assert_eq!(doc.get_text(), "Vaime");

    doc.local_insert(0, "Vuime ").unwrap();
    assert_eq!(doc.get_text(), "Vuime Vaime");

    doc.local_insert(11, "!").unwrap();
    assert_eq!(doc.get_text(), "Vuime Vaime!");

    doc.local_insert(5, ", :O").unwrap();
    assert_eq!(doc.get_text(), "Vuime, :O Vaime!");
}

#[test]
fn insert_middle_maintains_correct_origins() {
    let mut doc = Document::new(ClientId::new(1));

    doc.local_insert(0, "AC").unwrap();
    doc.local_insert(1, "B").unwrap();

    assert_eq!(doc.get_text(), "ABC");
    assert_eq!(doc.state_vector.get(&ClientId::new(1)), 3);

    let a_id = doc.head.unwrap();
    let a_block = doc.store.get(&a_id).unwrap();
    assert_eq!(a_block.content(), "A");

    let b_id = a_block.right().unwrap();
    let b_block = doc.store.get(&b_id).unwrap();
    assert_eq!(b_block.content(), "B");

    let c_id = b_block.right().unwrap();
    let c_block = doc.store.get(&c_id).unwrap();
    assert_eq!(c_block.content(), "C");
    assert_eq!(b_block.origin_left, Some(a_id));
    assert_eq!(b_block.origin_right, Some(c_id));
}

#[test]
fn split_block_in_middle_updates_links_and_content() {
    let (mut doc, id) = doc_with_single_block("hello");

    doc.split_block(id, 2).unwrap();

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

    doc.split_block(id, 0).unwrap();

    let block = doc.store.get(&id).unwrap();
    assert_eq!(block.content(), "hello");
    assert_eq!(block.right(), None);
}

#[test]
fn split_block_at_len_is_noop() {
    let (mut doc, id) = doc_with_single_block("hello");

    doc.split_block(id, 5).unwrap();

    let block = doc.store.get(&id).unwrap();
    assert_eq!(block.content(), "hello");
    assert_eq!(block.right(), None);
}

#[test]
fn split_block_past_len_is_noop() {
    let (mut doc, id) = doc_with_single_block("hello");

    doc.split_block(id, 99).unwrap();

    let block = doc.store.get(&id).unwrap();
    assert_eq!(block.content(), "hello");
    assert_eq!(block.right(), None);
}

#[test]
fn split_block_updates_existing_right_neighbor() {
    let (mut doc, left_id, right_id) = doc_with_two_blocks("abc", "def");

    doc.split_block(left_id, 1).unwrap();

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

    doc.split_block(id, 2).unwrap();

    let left = doc.store.get(&id).unwrap();
    let right_id = left.right().unwrap();
    let right = doc.store.get(&right_id).unwrap();

    assert!(left.is_deleted);
    assert!(right.is_deleted);
}

#[test]
fn get_block_and_offset_by_position_finds_first_block() {
    let (doc, left_id, _) = doc_with_two_blocks("abc", "def");

    let (found, offset, _tail) = doc.get_block_and_offset_by_position(2);

    assert_eq!(found, Some(left_id));
    assert_eq!(offset, 2);
}

#[test]
fn get_block_and_offset_by_position_finds_second_block() {
    let (doc, _, right_id) = doc_with_two_blocks("abc", "def");

    let (found, offset, _tail) = doc.get_block_and_offset_by_position(4);

    assert_eq!(found, Some(right_id));
    assert_eq!(offset, 1);
}

#[test]
fn get_block_and_offset_by_position_returns_none_past_end() {
    let (doc, _, _) = doc_with_two_blocks("abc", "def");

    let (found, offset, _tail) = doc.get_block_and_offset_by_position(7);

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

    let (found, offset, _tail) = doc.get_block_and_offset_by_position(2);

    assert_eq!(found, Some(right_id));
    assert_eq!(offset, 0);
}

#[test]
fn test_remote_insert_conflict_resolution() {
    let mut doc_a = Document::new(ClientId::new(1));
    let mut doc_b = Document::new(ClientId::new(2));

    doc_a.local_insert(0, "A").unwrap();

    let id_a = doc_a.head.unwrap();
    let block_a = doc_a.store.get(&id_a).unwrap().clone();
    doc_b.remote_insert(block_a).unwrap();

    assert_eq!(doc_a.get_text(), "A");
    assert_eq!(doc_b.get_text(), "A");

    doc_a.local_insert(1, "X").unwrap();
    doc_b.local_insert(1, "Y").unwrap();

    assert_eq!(doc_a.get_text(), "AX");
    assert_eq!(doc_b.get_text(), "AY");

    let id_x = BlockId::new(ClientId::new(1), Clock::new(1));
    let block_x = doc_a.store.get(&id_x).unwrap().clone();

    let id_y = BlockId::new(ClientId::new(2), Clock::new(0));
    let block_y = doc_b.store.get(&id_y).unwrap().clone();

    doc_a.remote_insert(block_y).unwrap();
    doc_b.remote_insert(block_x).unwrap();

    let final_text_a = doc_a.get_text();
    let final_text_b = doc_b.get_text();

    assert_eq!(final_text_a, final_text_b, "Documents failed to converge");
    assert_eq!(final_text_a, "AXY");
}
