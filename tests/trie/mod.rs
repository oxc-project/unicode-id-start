#![allow(clippy::module_inception)]

#[allow(dead_code, clippy::redundant_static_lifetimes, clippy::unreadable_literal)]
#[rustfmt::skip]
mod trie;

#[allow(unused_imports)]
#[cfg(test)]
pub(crate) use self::trie::*;
