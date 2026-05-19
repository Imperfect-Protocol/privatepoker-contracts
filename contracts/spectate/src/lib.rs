#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
#![cfg_attr(not(any(test, feature = "export-abi")), no_std)]

extern crate alloc;

pub mod privatepoker_spectate;

pub use privatepoker_spectate::*;
