#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
#![cfg_attr(not(any(test, feature = "export-abi")), no_std)]

#[macro_use]
extern crate alloc;

pub mod hash_to_curve;
pub mod privatepoker_hash_to_curve;

pub use privatepoker_hash_to_curve::*;
