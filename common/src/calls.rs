use alloc::vec::Vec;

use alloy_primitives::Address;
use alloy_sol_types::SolValue;
use stylus_sdk::{alloy_primitives::U256, stylus_core};

pub trait ContractCalls
where
    Self: stylus_core::HostAccess + stylus_core::storage::TopLevelStorage + Sized,
{
    #[inline]
    fn call_raw(&mut self, to: Address, calldata: &[u8]) -> Result<Vec<u8>, Vec<u8>> {
        self.vm().call(&self, to, calldata).map_err(Into::into)
    }

    #[inline]
    fn static_call_raw(&self, to: Address, calldata: &[u8]) -> Result<Vec<u8>, Vec<u8>> {
        self.vm()
            .static_call(&self, to, calldata)
            .map_err(Into::into)
    }

    #[inline]
    unsafe fn delegate_call_raw(
        &mut self,
        to: Address,
        calldata: &[u8],
    ) -> Result<Vec<u8>, Vec<u8>> {
        unsafe {
            self.vm()
                .delegate_call(&self, to, calldata)
                .map_err(Into::into)
        }
    }

    #[inline]
    fn call_bytes(&mut self, to: Address, calldata: &[u8], err: &[u8]) -> Result<Vec<u8>, Vec<u8>> {
        self.call_raw(to, calldata).map_err(|_| err.to_vec())
    }

    #[inline]
    fn static_call_bytes(
        &self,
        to: Address,
        calldata: &[u8],
        err: &[u8],
    ) -> Result<Vec<u8>, Vec<u8>> {
        self.static_call_raw(to, calldata).map_err(|_| err.to_vec())
    }

    #[inline]
    fn call_bool(&mut self, to: Address, calldata: &[u8], err: &[u8]) -> Result<(), Vec<u8>> {
        let output = self.call_bytes(to, calldata, err)?;
        let ok = bool::abi_decode(&output, true).map_err(|_| err.to_vec())?;
        if ok {
            Ok(())
        } else {
            Err(err.to_vec())
        }
    }

    #[inline]
    fn call_optional_bool(
        &mut self,
        to: Address,
        calldata: &[u8],
        err: &[u8],
    ) -> Result<(), Vec<u8>> {
        let output = self.call_bytes(to, calldata, err)?;
        if output.is_empty() {
            return Ok(());
        }
        let ok = bool::abi_decode(&output, true).map_err(|_| err.to_vec())?;
        if ok {
            Ok(())
        } else {
            Err(err.to_vec())
        }
    }

    #[inline]
    fn call_u256(&mut self, to: Address, calldata: &[u8], err: &[u8]) -> Result<U256, Vec<u8>> {
        let output = self.call_bytes(to, calldata, err)?;
        U256::abi_decode(&output, true).map_err(|_| err.to_vec())
    }

    #[inline]
    fn static_call_u256(&self, to: Address, calldata: &[u8], err: &[u8]) -> Result<U256, Vec<u8>> {
        let output = self.static_call_bytes(to, calldata, err)?;
        U256::abi_decode(&output, true).map_err(|_| err.to_vec())
    }
}

impl<T> ContractCalls for T where
    T: stylus_core::HostAccess + stylus_core::storage::TopLevelStorage + Sized
{
}
