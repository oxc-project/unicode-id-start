#![allow(clippy::let_underscore_untyped, clippy::unreadable_literal)]

use std::mem::size_of_val;

#[test]
fn test_size() {
    #[allow(dead_code)]
    #[path = "../src/tables.rs"]
    mod tables;

    let size = size_of_val(&tables::ASCII_START)
        + size_of_val(&tables::ASCII_CONTINUE)
        + size_of_val(&tables::TRIE_START)
        + size_of_val(&tables::TRIE_CONTINUE)
        + size_of_val(&tables::LEAF);
    assert_eq!(10336, size);
}

#[test]
fn test_id_size() {
    #[deny(dead_code)]
    #[path = "tables/mod.rs"]
    mod tables;

    let size = size_of_val(tables::ID_START) + size_of_val(tables::ID_CONTINUE);
    assert_eq!(11760, size);

    let _ = tables::BY_NAME;
}

#[cfg(target_pointer_width = "64")]
#[test]
fn test_trieset_size() {
    #[deny(dead_code)]
    #[allow(clippy::redundant_static_lifetimes)]
    #[path = "trie/trie.rs"]
    mod trie;

    let ucd_trie::TrieSet {
        tree1_level1,
        tree2_level1,
        tree2_level2,
        tree3_level1,
        tree3_level2,
        tree3_level3,
    } = *trie::ID_START;

    let start_size = size_of_val(trie::ID_START)
        + size_of_val(tree1_level1)
        + size_of_val(tree2_level1)
        + size_of_val(tree2_level2)
        + size_of_val(tree3_level1)
        + size_of_val(tree3_level2)
        + size_of_val(tree3_level3);

    let ucd_trie::TrieSet {
        tree1_level1,
        tree2_level1,
        tree2_level2,
        tree3_level1,
        tree3_level2,
        tree3_level3,
    } = *trie::ID_CONTINUE;

    let continue_size = size_of_val(trie::ID_CONTINUE)
        + size_of_val(tree1_level1)
        + size_of_val(tree2_level1)
        + size_of_val(tree2_level2)
        + size_of_val(tree3_level1)
        + size_of_val(tree3_level2)
        + size_of_val(tree3_level3);

    assert_eq!(10328, start_size + continue_size);

    let _ = trie::BY_NAME;
}

#[test]
fn test_fst_size() {
    let id_start_fst = include_bytes!("fst/id_start.fst");
    let id_continue_fst = include_bytes!("fst/id_continue.fst");
    let size = id_start_fst.len() + id_continue_fst.len();
    assert_eq!(142646, size);
}

#[test]
fn test_roaring_size() {
    #[path = "roaring/mod.rs"]
    mod roaring;

    let id_start_bitmap = roaring::id_start_bitmap();
    let id_continue_bitmap = roaring::id_continue_bitmap();
    let size = id_start_bitmap.serialized_size() + id_continue_bitmap.serialized_size();
    assert_eq!(66104, size);
}
