#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
#![cfg_attr(not(any(test, feature = "export-abi")), no_std)]

#[macro_use]
extern crate alloc;

pub mod privatepoker_aggregate_pub_key;

pub use privatepoker_aggregate_pub_key::*;
