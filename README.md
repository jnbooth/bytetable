# `bytetable`

High-performance data structures indexed by `u8`. Uses `#![no_std]`.

[![crates.io](https://img.shields.io/crates/v/bytetable.svg)](https://crates.io/crates/bytetable)
[![Documentation](https://docs.rs/bytetable/badge.svg)](https://docs.rs/bytetable)
![MIT licensed](https://img.shields.io/crates/l/bytetable.svg)
<br />
[![Dependency Status](https://deps.rs/crate/bytetable/latest/status.svg)](https://deps.rs/crate/bytetable)
![Downloads](https://img.shields.io/crates/d/bytetable.svg)

## Usage

To use `bytetable`, first add this to your `Cargo.toml`:

```toml
[dependencies]
bytetable = "1"
```

Next, add this to your crate:

```rust
use byte_table::ByteTable;
```

## no_std support

`bytetable` is completely implemented in the no_std environment. The `alloc` feature (enabled by default) adds support for creating data structures on the heap with `Box`. Passing `default-features = false` disables the `alloc` feature, removing its dependency on `extern crate alloc`.

## License

This project is licensed under the [MIT license](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in `bytetable` by you, shall be licensed as MIT, without any
additional terms or conditions.
