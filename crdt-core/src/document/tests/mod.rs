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

// ── delete() ─────────────────────────────────────────────────────────────────

#[test]
fn delete_entire_single_block() {
    let (mut doc, _) = doc_with_single_block("hello");
    doc.delete(0, 5);
    assert_eq!(visible_text(&doc), "");
}

#[test]
fn delete_prefix() {
    let (mut doc, _) = doc_with_single_block("hello");
    doc.delete(0, 2);
    assert_eq!(visible_text(&doc), "llo");
}

#[test]
fn delete_suffix() {
    let (mut doc, _) = doc_with_single_block("hello");
    doc.delete(3, 2);
    assert_eq!(visible_text(&doc), "hel");
}

#[test]
fn delete_middle() {
    let (mut doc, _) = doc_with_single_block("hello");
    doc.delete(1, 3);
    assert_eq!(visible_text(&doc), "ho");
}

#[test]
fn delete_zero_length_is_noop() {
    let (mut doc, id) = doc_with_single_block("hello");
    doc.delete(0, 0);
    assert!(!doc.store.get(&id).unwrap().is_deleted);
    assert_eq!(visible_text(&doc), "hello");
}

#[test]
fn delete_records_in_local_delete_set() {
    let (mut doc, id) = doc_with_single_block("hello");
    doc.delete(0, 5);
    assert!(doc.delete_set.contains(&id));
}

#[test]
fn delete_does_not_pollute_seen_delete_set() {
    let (mut doc, id) = doc_with_single_block("hello");
    doc.delete(0, 5);
    assert!(!doc.seen_delete_set.contains(&id));
}

#[test]
fn delete_split_records_correct_range_in_delete_set() {
    let (mut doc, _) = doc_with_single_block("hello");
    doc.delete(1, 3);
    assert!(doc.delete_set.contains(&bid(1, 1)));
    assert!(doc.delete_set.contains(&bid(1, 2)));
    assert!(doc.delete_set.contains(&bid(1, 3)));
    assert!(!doc.delete_set.contains(&bid(1, 0)));
    assert!(!doc.delete_set.contains(&bid(1, 4)));
}

#[test]
fn delete_skips_already_deleted_tombstones() {
    let cid = ClientId::new(1);
    let id_a = BlockId::new(cid, Clock::new(0));
    let id_b = BlockId::new(cid, Clock::new(1));
    let mut doc = Document::new(cid);

    let mut a = Block::new(id_a, None, Some(id_b), "a".to_string());
    a.set_right(Some(id_b));
    let mut b = Block::new(id_b, Some(id_a), None, "b".to_string());
    b.set_left(Some(id_a));

    doc.head = Some(id_a);
    doc.store.insert(a);
    doc.store.insert(b);

    doc.store.mark_deleted(&id_a);

    doc.delete(0, 1);

    assert!(doc.store.get(&id_b).unwrap().is_deleted);
    assert_eq!(visible_text(&doc), "");
}

#[test]
fn delete_across_two_blocks() {
    let (mut doc, _, _) = doc_with_two_blocks("abc", "def");
    doc.delete(1, 4); // "bcd e" → visible: "af"
    assert_eq!(visible_text(&doc), "af");
}

#[test]
fn delete_exactly_at_block_boundary() {
    // Deleting exactly the first block, nothing from the second.
    let (mut doc, left_id, right_id) = doc_with_two_blocks("abc", "def");
    doc.delete(0, 3);
    assert!(doc.store.get(&left_id).unwrap().is_deleted);
    assert!(!doc.store.get(&right_id).unwrap().is_deleted);
    assert_eq!(visible_text(&doc), "def");
}

#[test]
fn apply_remote_delete_set_marks_blocks() {
    let (mut doc, id) = doc_with_single_block("hello");

    let mut remote = DeleteSet::new();
    remote.add(id, 5);

    doc.apply_delete_set(&remote);

    assert!(doc.store.get(&id).unwrap().is_deleted);
}

#[test]
fn apply_delete_set_is_idempotent() {
    let (mut doc, id) = doc_with_single_block("hello");

    let mut remote = DeleteSet::new();
    remote.add(id, 5);

    doc.apply_delete_set(&remote);
    doc.apply_delete_set(&remote);

    assert!(doc.store.get(&id).unwrap().is_deleted);
}

#[test]
fn apply_delete_set_records_in_seen_not_local() {
    let (mut doc, id) = doc_with_single_block("hello");

    let mut remote = DeleteSet::new();
    remote.add(id, 5);

    doc.apply_delete_set(&remote);

    assert!(
        doc.seen_delete_set.contains(&id),
        "remote delete should be recorded in seen_delete_set"
    );
    assert!(
        !doc.delete_set.contains(&id),
        "remote delete must not pollute local delete_set"
    );
}

// ── collect_garbage() ────────────────────────────────────────────────────────

#[test]
fn collect_garbage_clears_content_of_confirmed_deleted_blocks() {
    let (mut doc, id) = doc_with_single_block("hello");
    doc.delete(0, 5);

    let confirmed = doc.delete_set.clone();
    doc.collect_garbage(&confirmed);

    let block = doc.store.get(&id).unwrap();
    assert!(block.is_deleted);
    assert!(block.is_empty(), "content should be cleared after GC");
}

#[test]
fn collect_garbage_preserves_block_len_after_clearing_content() {
    let (mut doc, id) = doc_with_single_block("hello");
    doc.delete(0, 5);

    let confirmed = doc.delete_set.clone();
    doc.collect_garbage(&confirmed);

    let block = doc.store.get(&id).unwrap();
    assert_eq!(block.len, 5, "len must be preserved after GC");
    assert!(block.is_empty(), "content must be cleared");
}

#[test]
fn collect_garbage_leaves_non_deleted_blocks_alone() {
    let (mut doc, _) = doc_with_single_block("hello");
    doc.delete(0, 2);

    let confirmed = doc.delete_set.clone();
    doc.collect_garbage(&confirmed);

    assert_eq!(visible_text(&doc), "llo");
}

#[test]
fn collect_garbage_does_not_affect_unconfirmed_blocks() {
    let (mut doc, left_id, right_id) = doc_with_two_blocks("he", "llo");
    doc.delete(0, 2);
    doc.delete(2, 3);

    let mut confirmed = DeleteSet::new();
    confirmed.add(left_id, 2);
    doc.collect_garbage(&confirmed);

    assert!(doc.store.get(&left_id).unwrap().is_empty());
    assert!(!doc.store.get(&right_id).unwrap().is_empty());
}
