#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation, // https://github.com/rust-lang/rust-clippy/issues/9613
    clippy::let_underscore_untyped,
    clippy::match_wild_err_arm,
    clippy::module_name_repetitions,
    clippy::too_many_lines,
    clippy::uninlined_format_args
)]

mod output;
mod parse;
mod write;

use crate::parse::parse_id_properties;
use std::collections::{BTreeMap as Map, VecDeque};
use std::convert::TryFrom;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;

const CHUNK: usize = 64;
const UCD: &str = "UCD";
const TABLES: &str = "src/tables.rs";

fn main() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let unicode_ident_dir = manifest_dir.parent().unwrap();
    let ucd_dir = unicode_ident_dir.join(UCD);
    let properties = parse_id_properties(&ucd_dir);

    let mut chunkmap = Map::<[u8; CHUNK], u8>::new();
    let mut dense = Vec::<[u8; CHUNK]>::new();
    let mut new_chunk = |chunk| {
        if let Some(prev) = chunkmap.get(&chunk) {
            *prev
        } else {
            dense.push(chunk);
            let Ok(new) = u8::try_from(chunkmap.len()) else {
                panic!("exceeded 256 unique chunks");
            };
            chunkmap.insert(chunk, new);
            new
        }
    };

    let empty_chunk = [0u8; CHUNK];
    new_chunk(empty_chunk);

    let mut index_start = Vec::<u8>::new();
    let mut index_continue = Vec::<u8>::new();
    for i in 0..(u32::from(char::MAX) + 1) / CHUNK as u32 / 8 {
        let mut start_bits = empty_chunk;
        let mut continue_bits = empty_chunk;
        for j in 0..CHUNK as u32 {
            let this_start = &mut start_bits[j as usize];
            let this_continue = &mut continue_bits[j as usize];
            for k in 0..8u32 {
                let code = (i * CHUNK as u32 + j) * 8 + k;
                if code >= 0x80 {
                    if let Some(ch) = char::from_u32(code) {
                        *this_start |= (properties.is_id_start(ch) as u8) << k;
                        *this_continue |= (properties.is_id_continue(ch) as u8) << k;
                    }
                }
            }
        }
        index_start.push(new_chunk(start_bits));
        index_continue.push(new_chunk(continue_bits));
    }

    let id_start_high = split_high(&mut index_start, |ch| properties.is_id_start(ch));
    let id_continue_high = split_high(&mut index_continue, |ch| properties.is_id_continue(ch));

    let mut halfchunkmap = Map::new();
    for chunk in &dense {
        let mut front = [0u8; CHUNK / 2];
        let mut back = [0u8; CHUNK / 2];
        front.copy_from_slice(&chunk[..CHUNK / 2]);
        back.copy_from_slice(&chunk[CHUNK / 2..]);
        halfchunkmap
            .entry(front)
            .or_insert_with(VecDeque::new)
            .push_back(back);
    }

    let mut halfdense = Vec::<u8>::new();
    let mut dense_to_halfdense = Map::<u8, u8>::new();
    for chunk in &dense {
        let original_pos = chunkmap[chunk];
        if dense_to_halfdense.contains_key(&original_pos) {
            continue;
        }
        let mut front = [0u8; CHUNK / 2];
        let mut back = [0u8; CHUNK / 2];
        front.copy_from_slice(&chunk[..CHUNK / 2]);
        back.copy_from_slice(&chunk[CHUNK / 2..]);
        dense_to_halfdense.insert(
            original_pos,
            match u8::try_from(halfdense.len() / (CHUNK / 2)) {
                Ok(byte) => byte,
                Err(_) => panic!("exceeded 256 half-chunks"),
            },
        );
        halfdense.extend_from_slice(&front);
        halfdense.extend_from_slice(&back);
        while let Some(next) = halfchunkmap.get_mut(&back).and_then(VecDeque::pop_front) {
            let mut concat = empty_chunk;
            concat[..CHUNK / 2].copy_from_slice(&back);
            concat[CHUNK / 2..].copy_from_slice(&next);
            let original_pos = chunkmap[&concat];
            if dense_to_halfdense.contains_key(&original_pos) {
                continue;
            }
            dense_to_halfdense.insert(
                original_pos,
                match u8::try_from(halfdense.len() / (CHUNK / 2) - 1) {
                    Ok(byte) => byte,
                    Err(_) => panic!("exceeded 256 half-chunks"),
                },
            );
            halfdense.extend_from_slice(&next);
            back = next;
        }
    }

    for index in &mut index_start {
        *index = dense_to_halfdense[index];
    }
    for index in &mut index_continue {
        *index = dense_to_halfdense[index];
    }

    let out = write::output(
        &properties,
        &index_start,
        &index_continue,
        &halfdense,
        &id_start_high,
        &id_continue_high,
    );
    let path = unicode_ident_dir.join(TABLES);
    if let Err(err) = fs::write(&path, out) {
        let _ = writeln!(io::stderr(), "{}: {err}", path.display());
        process::exit(1);
    }
}

/// Trim the trailing part of a first-level trie index and return any high content as ranges.
///
/// After dropping trailing empty (all-zero) entries, an index can still be very long because a
/// small amount of content lives high in the codepoint space, separated from the main body by a
/// large run of empty entries. For example `ID_Continue` reaches the variation selectors at
/// U+E0100..=U+E01EF, ~1.3K empty entries above the rest of the table, which forces the whole
/// index to span that gap.
///
/// If such a gap exists, truncate the index at it and return the codepoints beyond it as explicit
/// `(lo, hi)` ranges, to be tested separately in the lookup. This drops the interior run of empty
/// entries from the baked table. The lookup treats `index.len() * CHUNK * 8` as the boundary: any
/// codepoint at or above it is matched against these ranges instead of the trie.
fn split_high(index: &mut Vec<u8>, is_set: impl Fn(char) -> bool) -> Vec<(u32, u32)> {
    // Minimum length (in entries) of an interior empty run that is worth splitting out.
    const GAP: usize = 256;

    // Drop trailing empty entries.
    while let Some(&0) = index.last() {
        index.pop();
    }

    // Find the longest run of empty entries.
    let (mut best_start, mut best_len) = (0usize, 0usize);
    let mut i = 0;
    while i < index.len() {
        if index[i] != 0 {
            i += 1;
            continue;
        }
        let start = i;
        while i < index.len() && index[i] == 0 {
            i += 1;
        }
        if i - start > best_len {
            best_start = start;
            best_len = i - start;
        }
    }

    // Only split when the gap is large and there is content beyond it.
    if best_len < GAP || best_start + best_len >= index.len() {
        return Vec::new();
    }

    // Entries before the gap stay in the trie; everything at or above `boundary` becomes a range.
    let boundary = (best_start * CHUNK * 8) as u32;
    index.truncate(best_start);

    let mut ranges = Vec::new();
    let mut cp = boundary;
    while cp <= u32::from(char::MAX) {
        if !char::from_u32(cp).is_some_and(&is_set) {
            cp += 1;
            continue;
        }
        let lo = cp;
        while cp <= u32::from(char::MAX) && char::from_u32(cp).is_some_and(&is_set) {
            cp += 1;
        }
        ranges.push((lo, cp - 1));
    }
    ranges
}
