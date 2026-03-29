//! High-performance data structures indexed by `u8`.
//!
//! # Description
//!
//! This crate provides high-performance data structures for operations where
//! the domain is all of `u8`, that is, every integer between 0 and 255
//! (inclusive).
//!
//! - [`ByteSet`]: Like `HashSet<u8>`, but implemented as a bit array. Nearly
//!   all operations are `const fn`.
//! - [`ByteTable`]: Lookup table implemented as `[T; 256]`. Eliminates the need
//!   for bounds checking. Useful for caching expensive operations on `u8`s.

#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod set;
pub use set::ByteSet;

pub mod table;
pub use table::ByteTable;
