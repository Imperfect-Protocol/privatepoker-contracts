use alloy_primitives::U256;
use stylus_sdk::host::VM;
use stylus_sdk::storage::StorageType;

const SLOT_BYTE_SPACE: u8 = 32;

/// Code borrowed from openzeppelin (rust-contracts-stylus)
pub struct StorageSlot;

impl StorageSlot {
    /// Returns a [`StorageType`] located at `slot`.
    ///
    /// # Arguments
    ///
    /// * `slot` - The slot to get the address from.
    #[must_use]
    pub fn get_slot<ST: StorageType>(slot: impl Into<U256>) -> ST {
        #[cfg(not(any(target_arch = "wasm32", feature = "export-abi")))]
        let host = VM(stylus_sdk::host::WasmVM {});

        #[cfg(any(target_arch = "wasm32", feature = "export-abi"))]
        let host = VM(stylus_sdk::host::WasmVM {});

        #[allow(clippy::cast_possible_truncation)]
        unsafe {
            ST::new(slot.into(), SLOT_BYTE_SPACE - ST::SLOT_BYTES as u8, host)
        }
    }
}
