//! # CLVM Traits
//! This is a library for encoding and decoding Rust values using a CLVM allocator.
//! It provides implementations for every fixed-width signed and unsigned integer type,
//! as well as many other values in the standard library that would be common to encode.
//!
//! As well as the built-in implementations, this library exposes two derive macros
//! for implementing the `ToClvm` and `FromClvm` traits on structs. They be marked
//! with one of the following encodings:
//!
//! * `#[clvm(tuple)]` for unterminated lists such as `(A . (B . C))`.
//! * `#[clvm(proper_list)]` for proper lists such as `(A B C)`, or in other words `(A . (B . (C . ())))`.
//! * `#[clvm(curried_args)]` for curried arguments such as `(c (q . A) (c (q . B) (c (q . C) 1)))`.

#![cfg_attr(
    feature = "derive",
    doc = r#"
## Derive Example

```rust
use clvmr::Allocator;
use clvm_traits::{ToClvm, FromClvm};

#[derive(Debug, PartialEq, Eq, ToClvm, FromClvm)]
#[clvm(tuple)]
struct Point {
    x: i32,
    y: i32,
}

let a = &mut Allocator::new();

let point = Point { x: 5, y: 2 };
let ptr = point.to_clvm(a).unwrap();

assert_eq!(Point::from_clvm(a, ptr).unwrap(), point);
```
"#
)]

#[cfg(feature = "derive")]
pub use clvm_derive::*;

mod error;
mod from_clvm;
mod macros;
mod match_byte;
mod to_clvm;

pub use error::*;
pub use from_clvm::*;
pub use macros::*;
pub use match_byte::*;
pub use to_clvm::*;

#[cfg(test)]
#[cfg(feature = "derive")]
mod tests {
    extern crate self as clvm_traits;

    use std::fmt;

    use clvmr::{serde::node_to_bytes, Allocator};

    use super::*;

    #[derive(Debug, ToClvm, FromClvm, PartialEq, Eq)]
    #[clvm(tuple)]
    struct TupleStruct {
        a: u64,
        b: i32,
    }

    #[derive(Debug, ToClvm, FromClvm, PartialEq, Eq)]
    #[clvm(proper_list)]
    struct ProperListStruct {
        a: u64,
        b: i32,
    }

    #[derive(Debug, ToClvm, FromClvm, PartialEq, Eq)]
    #[clvm(curried_args)]
    struct CurriedArgsStruct {
        a: u64,
        b: i32,
    }

    fn check<T>(value: T, expected: &str)
    where
        T: fmt::Debug + PartialEq + ToClvm + FromClvm,
    {
        let a = &mut Allocator::new();

        let ptr = value.to_clvm(a).unwrap();
        let round_trip = T::from_clvm(a, ptr).unwrap();
        assert_eq!(value, round_trip);

        let bytes = node_to_bytes(a, ptr).unwrap();
        let actual = hex::encode(bytes);
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_tuple() {
        check(TupleStruct { a: 52, b: -32 }, "ff3481e0");
    }

    #[test]
    fn test_proper_list() {
        check(ProperListStruct { a: 52, b: -32 }, "ff34ff81e080");
    }

    #[test]
    fn test_curried_args() {
        check(
            CurriedArgsStruct { a: 52, b: -32 },
            "ff04ffff0134ffff04ffff0181e0ff018080",
        );
    }
}