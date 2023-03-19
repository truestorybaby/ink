// Copyright 2018-2022 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! External C API to communicate with substrate contracts runtime module.
//!
//! Refer to substrate FRAME contract module for more documentation.

use crate::ReturnFlags;
use core::marker::PhantomData;

macro_rules! define_error_codes {
    (
        $(
            $( #[$attr:meta] )*
            $name:ident = $discr:literal,
        )*
    ) => {
        /// Every error that can be returned to a contract when it calls any of the host functions.
        #[repr(u32)]
        pub enum Error {
            $(
                $( #[$attr] )*
                $name = $discr,
            )*
            /// Returns if an unknown error was received from the host module.
            Unknown,
        }

        impl From<ReturnCode> for Result {
            #[inline]
            fn from(return_code: ReturnCode) -> Self {
                match return_code.0 {
                    0 => Ok(()),
                    $(
                        $discr => Err(Error::$name),
                    )*
                    _ => Err(Error::Unknown),
                }
            }
        }
    };
}
define_error_codes! {
    /// The called function trapped and has its state changes reverted.
    /// In this case no output buffer is returned.
    /// Can only be returned from `call` and `instantiate`.
    CalleeTrapped = 1,
    /// The called function ran to completion but decided to revert its state.
    /// An output buffer is returned when one was supplied.
    /// Can only be returned from `call` and `instantiate`.
    CalleeReverted = 2,
    /// The passed key does not exist in storage.
    KeyNotFound = 3,
    /// Deprecated and no longer returned: There is only the minimum balance.
    _BelowSubsistenceThreshold = 4,
    /// Transfer failed for other not further specified reason. Most probably
    /// reserved or locked balance of the sender that was preventing the transfer.
    TransferFailed = 5,
    /// Deprecated and no longer returned: Endowment is no longer required.
    _EndowmentTooLow = 6,
    /// No code could be found at the supplied code hash.
    CodeNotFound = 7,
    /// The account that was called is no contract.
    NotCallable = 8,
    /// The call to `debug_message` had no effect because debug message
    /// recording was disabled.
    LoggingDisabled = 9,
    /// The call dispatched by `call_runtime` was executed but returned an error.
    CallRuntimeFailed = 10,
    /// ECDSA public key recovery failed. Most probably wrong recovery id or signature.
    EcdsaRecoveryFailed = 11,
}

/// Thin-wrapper around a `u32` representing a pointer for Wasm32.
///
/// Only for shared references.
///
/// # Note
///
/// Can only be constructed from shared reference types and encapsulates the
/// conversion from reference to raw `u32`.
/// Does not allow accessing the internal `u32` value.
#[derive(Debug)]
#[repr(transparent)]
pub struct Ptr32<'a, T>
where
    T: ?Sized,
{
    /// The internal Wasm32 raw pointer value.
    ///
    /// Must not be readable or directly usable by any safe Rust code.
    _value: u32,
    /// We handle types like these as if the associated lifetime was exclusive.
    marker: PhantomData<fn() -> &'a T>,
}

impl<'a, T> Ptr32<'a, T>
where
    T: ?Sized,
{
    /// Creates a new Wasm32 pointer for the given raw pointer value.
    fn new(value: u32) -> Self {
        Self {
            _value: value,
            marker: Default::default(),
        }
    }
}

impl<'a, T> Ptr32<'a, [T]> {
    /// Creates a new Wasm32 pointer from the given shared slice.
    pub fn from_slice(slice: &'a [T]) -> Self {
        Self::new(slice.as_ptr() as u32)
    }
}

/// Thin-wrapper around a `u32` representing a pointer for Wasm32.
///
/// Only for exclusive references.
///
/// # Note
///
/// Can only be constructed from exclusive reference types and encapsulates the
/// conversion from reference to raw `u32`.
/// Does not allow accessing the internal `u32` value.
#[derive(Debug)]
#[repr(transparent)]
pub struct Ptr32Mut<'a, T>
where
    T: ?Sized,
{
    /// The internal Wasm32 raw pointer value.
    ///
    /// Must not be readable or directly usable by any safe Rust code.
    _value: u32,
    /// We handle types like these as if the associated lifetime was exclusive.
    marker: PhantomData<fn() -> &'a mut T>,
}

impl<'a, T> Ptr32Mut<'a, T>
where
    T: ?Sized,
{
    /// Creates a new Wasm32 pointer for the given raw pointer value.
    fn new(value: u32) -> Self {
        Self {
            _value: value,
            marker: Default::default(),
        }
    }
}

impl<'a, T> Ptr32Mut<'a, [T]> {
    /// Creates a new Wasm32 pointer from the given exclusive slice.
    pub fn from_slice(slice: &'a mut [T]) -> Self {
        Self::new(slice.as_ptr() as u32)
    }
}

impl<'a, T> Ptr32Mut<'a, T>
where
    T: Sized,
{
    /// Creates a new Wasm32 pointer from the given exclusive reference.
    pub fn from_ref(a_ref: &'a mut T) -> Self {
        let a_ptr: *mut T = a_ref;
        Self::new(a_ptr as u32)
    }
}

/// Used as a sentinel value when reading and writing contract memory.
///
/// We use this value to signal `None` to a contract when only a primitive is allowed
/// and we don't want to go through encoding a full Rust type. Using `u32::Max` is a safe
/// sentinel because contracts are never allowed to use such a large amount of resources.
/// So this value doesn't make sense for a memory location or length.
const SENTINEL: u32 = u32::MAX;

/// The raw return code returned by the host side.
#[repr(transparent)]
pub struct ReturnCode(u32);

impl From<ReturnCode> for Option<u32> {
    fn from(code: ReturnCode) -> Self {
        (code.0 < SENTINEL).then_some(code.0)
    }
}

impl ReturnCode {
    /// Returns the raw underlying `u32` representation.
    pub fn into_u32(self) -> u32 {
        self.0
    }
    /// Returns the underlying `u32` converted into `bool`.
    pub fn into_bool(self) -> bool {
        self.0.ne(&0)
    }
}

type Result = core::result::Result<(), Error>;

mod sys {
    use super::{
        Ptr32,
        Ptr32Mut,
        ReturnCode,
    };

    #[link(wasm_import_module = "seal0")]
    extern "C" {
        pub fn transfer(
            account_id_ptr: Ptr32<[u8]>,
            account_id_len: u32,
            transferred_value_ptr: Ptr32<[u8]>,
            transferred_value_len: u32,
        ) -> ReturnCode;

        pub fn deposit_event(
            topics_ptr: Ptr32<[u8]>,
            topics_len: u32,
            data_ptr: Ptr32<[u8]>,
            data_len: u32,
        );

        pub fn call_chain_extension(
            func_id: u32,
            input_ptr: Ptr32<[u8]>,
            input_len: u32,
            output_ptr: Ptr32Mut<[u8]>,
            output_len_ptr: Ptr32Mut<u32>,
        ) -> ReturnCode;

        pub fn input(buf_ptr: Ptr32Mut<[u8]>, buf_len_ptr: Ptr32Mut<u32>);
        pub fn seal_return(flags: u32, data_ptr: Ptr32<[u8]>, data_len: u32) -> !;

        pub fn caller(output_ptr: Ptr32Mut<[u8]>, output_len_ptr: Ptr32Mut<u32>);
        pub fn block_number(output_ptr: Ptr32Mut<[u8]>, output_len_ptr: Ptr32Mut<u32>);
        pub fn address(output_ptr: Ptr32Mut<[u8]>, output_len_ptr: Ptr32Mut<u32>);
        pub fn balance(output_ptr: Ptr32Mut<[u8]>, output_len_ptr: Ptr32Mut<u32>);
        pub fn weight_to_fee(
            gas: u64,
            output_ptr: Ptr32Mut<[u8]>,
            output_len_ptr: Ptr32Mut<u32>,
        );
        pub fn gas_left(output_ptr: Ptr32Mut<[u8]>, output_len_ptr: Ptr32Mut<u32>);
        pub fn value_transferred(
            output_ptr: Ptr32Mut<[u8]>,
            output_len_ptr: Ptr32Mut<u32>,
        );
        pub fn now(output_ptr: Ptr32Mut<[u8]>, output_len_ptr: Ptr32Mut<u32>);
        pub fn minimum_balance(output_ptr: Ptr32Mut<[u8]>, output_len_ptr: Ptr32Mut<u32>);

        pub fn hash_keccak_256(
            input_ptr: Ptr32<[u8]>,
            input_len: u32,
            output_ptr: Ptr32Mut<[u8]>,
        );
        pub fn hash_blake2_256(
            input_ptr: Ptr32<[u8]>,
            input_len: u32,
            output_ptr: Ptr32Mut<[u8]>,
        );
        pub fn hash_blake2_128(
            input_ptr: Ptr32<[u8]>,
            input_len: u32,
            output_ptr: Ptr32Mut<[u8]>,
        );
        pub fn hash_sha2_256(
            input_ptr: Ptr32<[u8]>,
            input_len: u32,
            output_ptr: Ptr32Mut<[u8]>,
        );

        pub fn is_contract(account_id_ptr: Ptr32<[u8]>) -> ReturnCode;

        pub fn caller_is_origin() -> ReturnCode;

        pub fn set_code_hash(code_hash_ptr: Ptr32<[u8]>) -> ReturnCode;

        pub fn code_hash(
            account_id_ptr: Ptr32<[u8]>,
            output_ptr: Ptr32Mut<[u8]>,
            output_len_ptr: Ptr32Mut<u32>,
        ) -> ReturnCode;

        pub fn own_code_hash(output_ptr: Ptr32Mut<[u8]>, output_len_ptr: Ptr32Mut<u32>);

        #[cfg(feature = "ink-debug")]
        pub fn debug_message(str_ptr: Ptr32<[u8]>, str_len: u32) -> ReturnCode;

        pub fn delegate_call(
            flags: u32,
            code_hash_ptr: Ptr32<[u8]>,
            input_data_ptr: Ptr32<[u8]>,
            input_data_len: u32,
            output_ptr: Ptr32Mut<[u8]>,
            output_len_ptr: Ptr32Mut<u32>,
        ) -> ReturnCode;

        pub fn ecdsa_recover(
            // 65 bytes of ecdsa signature
            signature_ptr: Ptr32<[u8]>,
            // 32 bytes hash of the message
            message_hash_ptr: Ptr32<[u8]>,
            output_ptr: Ptr32Mut<[u8]>,
        ) -> ReturnCode;

        pub fn ecdsa_to_eth_address(
            public_key_ptr: Ptr32<[u8]>,
            output_ptr: Ptr32Mut<[u8]>,
        ) -> ReturnCode;

        pub fn take_storage(
            key_ptr: Ptr32<[u8]>,
            key_len: u32,
            out_ptr: Ptr32Mut<[u8]>,
            out_len_ptr: Ptr32Mut<u32>,
        ) -> ReturnCode;

        #[cfg(feature = "call-runtime")]
        pub fn call_runtime(call_ptr: Ptr32<[u8]>, call_len: u32) -> ReturnCode;
    }

    #[link(wasm_import_module = "seal1")]
    extern "C" {
        pub fn instantiate(
            init_code_ptr: Ptr32<[u8]>,
            gas: u64,
            endowment_ptr: Ptr32<[u8]>,
            input_ptr: Ptr32<[u8]>,
            input_len: u32,
            address_ptr: Ptr32Mut<[u8]>,
            address_len_ptr: Ptr32Mut<u32>,
            output_ptr: Ptr32Mut<[u8]>,
            output_len_ptr: Ptr32Mut<u32>,
            salt_ptr: Ptr32<[u8]>,
            salt_len: u32,
        ) -> ReturnCode;

        pub fn terminate(beneficiary_ptr: Ptr32<[u8]>) -> !;

        pub fn call(
            flags: u32,
            callee_ptr: Ptr32<[u8]>,
            gas: u64,
            transferred_value_ptr: Ptr32<[u8]>,
            input_data_ptr: Ptr32<[u8]>,
            input_data_len: u32,
            output_ptr: Ptr32Mut<[u8]>,
            output_len_ptr: Ptr32Mut<u32>,
        ) -> ReturnCode;

        // # Parameters
        //
        // - `key_ptr`: pointer into the linear memory where the key is placed.
        // - `key_len`: the length of the key in bytes.
        //
        // # Return Value
        //
        // Returns the size of the pre-existing value at the specified key if any. Otherwise
        // `SENTINEL` is returned as a sentinel value.
        pub fn clear_storage(key_ptr: Ptr32<[u8]>, key_len: u32) -> ReturnCode;

        // # Parameters
        //
        // - `key_ptr`: pointer into the linear memory where the key of the requested value is placed.
        // - `key_len`: the length of the key in bytes.
        //
        // # Return Value
        //
        // Returns the size of the pre-existing value at the specified key if any. Otherwise
        // `SENTINEL` is returned as a sentinel value.
        pub fn contains_storage(key_ptr: Ptr32<[u8]>, key_len: u32) -> ReturnCode;

        // # Parameters
        //
        // - `key_ptr`: pointer into the linear memory where the key of the requested value is placed.
        // - `key_len`: the length of the key in bytes.
        // - `out_ptr`: pointer to the linear memory where the value is written to.
        // - `out_len_ptr`: in-out pointer into linear memory where the buffer length
        //   is read from and the value length is written to.
        //
        // # Errors
        //
        // `ReturnCode::KeyNotFound`
        pub fn get_storage(
            key_ptr: Ptr32<[u8]>,
            key_len: u32,
            out_ptr: Ptr32Mut<[u8]>,
            out_len_ptr: Ptr32Mut<u32>,
        ) -> ReturnCode;
    }

    #[link(wasm_import_module = "seal2")]
    extern "C" {
        // # Parameters
        //
        // - `key_ptr`: pointer into the linear memory where the location to store the value is placed.
        // - `key_len`: the length of the key in bytes.
        // - `value_ptr`: pointer into the linear memory where the value to set is placed.
        // - `value_len`: the length of the value in bytes.
        //
        // # Return Value
        //
        // Returns the size of the pre-existing value at the specified key if any. Otherwise
        // `SENTINEL` is returned as a sentinel value.
        pub fn set_storage(
            key_ptr: Ptr32<[u8]>,
            key_len: u32,
            value_ptr: Ptr32<[u8]>,
            value_len: u32,
        ) -> ReturnCode;
    }
}

#[inline(always)]
fn extract_from_slice(output: &mut &mut [u8], new_len: usize) {
    debug_assert!(new_len <= output.len());
    let tmp = core::mem::take(output);
    *output = &mut tmp[..new_len];
}

#[inline(always)]
pub fn instantiate(
    code_hash: &[u8],
    gas_limit: u64,
    endowment: &[u8],
    input: &[u8],
    out_address: &mut &mut [u8],
    out_return_value: &mut &mut [u8],
    salt: &[u8],
) -> Result {
    let mut address_len = out_address.len() as u32;
    let mut return_value_len = out_return_value.len() as u32;
    let ret_code = {
        unsafe {
            sys::instantiate(
                Ptr32::from_slice(code_hash),
                gas_limit,
                Ptr32::from_slice(endowment),
                Ptr32::from_slice(input),
                input.len() as u32,
                Ptr32Mut::from_slice(out_address),
                Ptr32Mut::from_ref(&mut address_len),
                Ptr32Mut::from_slice(out_return_value),
                Ptr32Mut::from_ref(&mut return_value_len),
                Ptr32::from_slice(salt),
                salt.len() as u32,
            )
        }
    };
    extract_from_slice(out_address, address_len as usize);
    extract_from_slice(out_return_value, return_value_len as usize);
    ret_code.into()
}

#[inline(always)]
pub fn call(
    flags: u32,
    callee: &[u8],
    gas_limit: u64,
    value: &[u8],
    input: &[u8],
    output: &mut &mut [u8],
) -> Result {
    let mut output_len = output.len() as u32;
    let ret_code = {
        unsafe {
            sys::call(
                flags,
                Ptr32::from_slice(callee),
                gas_limit,
                Ptr32::from_slice(value),
                Ptr32::from_slice(input),
                input.len() as u32,
                Ptr32Mut::from_slice(output),
                Ptr32Mut::from_ref(&mut output_len),
            )
        }
    };
    extract_from_slice(output, output_len as usize);
    ret_code.into()
}

#[inline(always)]
pub fn delegate_call(
    flags: u32,
    code_hash: &[u8],
    input: &[u8],
    output: &mut &mut [u8],
) -> Result {
    let mut output_len = output.len() as u32;
    let ret_code = {
        unsafe {
            sys::delegate_call(
                flags,
                Ptr32::from_slice(code_hash),
                Ptr32::from_slice(input),
                input.len() as u32,
                Ptr32Mut::from_slice(output),
                Ptr32Mut::from_ref(&mut output_len),
            )
        }
    };
    extract_from_slice(output, output_len as usize);
    ret_code.into()
}

pub fn transfer(account_id: &[u8], value: &[u8]) -> Result {
    let ret_code = unsafe {
        sys::transfer(
            Ptr32::from_slice(account_id),
            account_id.len() as u32,
            Ptr32::from_slice(value),
            value.len() as u32,
        )
    };
    ret_code.into()
}

pub fn deposit_event(topics: &[u8], data: &[u8]) {
    unsafe {
        sys::deposit_event(
            Ptr32::from_slice(topics),
            topics.len() as u32,
            Ptr32::from_slice(data),
            data.len() as u32,
        )
    }
}

pub fn set_storage(key: &[u8], encoded_value: &[u8]) -> Option<u32> {
    let ret_code = unsafe {
        sys::set_storage(
            Ptr32::from_slice(key),
            key.len() as u32,
            Ptr32::from_slice(encoded_value),
            encoded_value.len() as u32,
        )
    };
    ret_code.into()
}

pub fn clear_storage(key: &[u8]) -> Option<u32> {
    let ret_code =
        unsafe { sys::clear_storage(Ptr32::from_slice(key), key.len() as u32) };
    ret_code.into()
}

#[inline(always)]
pub fn get_storage(key: &[u8], output: &mut &mut [u8]) -> Result {
    let mut output_len = output.len() as u32;
    let ret_code = {
        unsafe {
            sys::get_storage(
                Ptr32::from_slice(key),
                key.len() as u32,
                Ptr32Mut::from_slice(output),
                Ptr32Mut::from_ref(&mut output_len),
            )
        }
    };
    extract_from_slice(output, output_len as usize);
    ret_code.into()
}

#[inline(always)]
pub fn take_storage(key: &[u8], output: &mut &mut [u8]) -> Result {
    let mut output_len = output.len() as u32;
    let ret_code = {
        unsafe {
            sys::take_storage(
                Ptr32::from_slice(key),
                key.len() as u32,
                Ptr32Mut::from_slice(output),
                Ptr32Mut::from_ref(&mut output_len),
            )
        }
    };
    extract_from_slice(output, output_len as usize);
    ret_code.into()
}

pub fn storage_contains(key: &[u8]) -> Option<u32> {
    let ret_code =
        unsafe { sys::contains_storage(Ptr32::from_slice(key), key.len() as u32) };
    ret_code.into()
}

pub fn terminate(beneficiary: &[u8]) -> ! {
    unsafe { sys::terminate(Ptr32::from_slice(beneficiary)) }
}

#[inline(always)]
pub fn call_chain_extension(func_id: u32, input: &[u8], output: &mut &mut [u8]) -> u32 {
    let mut output_len = output.len() as u32;
    let ret_code = {
        unsafe {
            sys::call_chain_extension(
                func_id,
                Ptr32::from_slice(input),
                input.len() as u32,
                Ptr32Mut::from_slice(output),
                Ptr32Mut::from_ref(&mut output_len),
            )
        }
    };
    extract_from_slice(output, output_len as usize);
    ret_code.into_u32()
}

#[inline(always)]
pub fn input(output: &mut &mut [u8]) {
    let mut output_len = output.len() as u32;
    {
        unsafe {
            sys::input(
                Ptr32Mut::from_slice(output),
                Ptr32Mut::from_ref(&mut output_len),
            )
        };
    }
    extract_from_slice(output, output_len as usize);
}

pub fn return_value(flags: ReturnFlags, return_value: &[u8]) -> ! {
    unsafe {
        sys::seal_return(
            flags.into_u32(),
            Ptr32::from_slice(return_value),
            return_value.len() as u32,
        )
    }
}

#[cfg(feature = "call-runtime")]
pub fn call_runtime(call: &[u8]) -> Result {
    let ret_code =
        unsafe { sys::call_runtime(Ptr32::from_slice(call), call.len() as u32) };
    ret_code.into()
}

macro_rules! impl_wrapper_for {
    ( $( $name:ident, )* ) => {
        $(
            #[inline(always)]
            pub fn $name(output: &mut &mut [u8]) {
                let mut output_len = output.len() as u32;
                {
                    unsafe {
                        sys::$name(
                            Ptr32Mut::from_slice(output),
                            Ptr32Mut::from_ref(&mut output_len),
                        )
                    };
                }
            }
        )*
    }
}
impl_wrapper_for! {
    caller,
    block_number,
    address,
    balance,
    gas_left,
    value_transferred,
    now,
    minimum_balance,
}

#[inline(always)]
pub fn weight_to_fee(gas: u64, output: &mut &mut [u8]) {
    let mut output_len = output.len() as u32;
    {
        unsafe {
            sys::weight_to_fee(
                gas,
                Ptr32Mut::from_slice(output),
                Ptr32Mut::from_ref(&mut output_len),
            )
        };
    }
    extract_from_slice(output, output_len as usize);
}

#[cfg(feature = "ink-debug")]
/// Call `debug_message` with the supplied UTF-8 encoded message.
///
/// If debug message recording is disabled in the contracts pallet, the first call will
/// return a `LoggingDisabled` error, and further calls will be a no-op to avoid the cost
/// of calling into the supervisor.
///
/// # Note
///
/// This depends on the `debug_message` interface which requires the
/// `"pallet-contracts/unstable-interface"` feature to be enabled in the target runtime.
pub fn debug_message(message: &str) {
    static mut DEBUG_ENABLED: bool = false;
    static mut FIRST_RUN: bool = true;

    // SAFETY: safe because executing in a single threaded context
    // We need those two variables in order to make sure that the assignment is performed
    // in the "logging enabled" case. This is because during RPC execution logging might
    // be enabled while it is disabled during the actual execution as part of a transaction.
    // The gas estimation takes place during RPC execution. We want to overestimate instead
    // of underestimate gas usage. Otherwise using this estimate could lead to a out of gas error.
    if unsafe { DEBUG_ENABLED || FIRST_RUN } {
        let bytes = message.as_bytes();
        let ret_code =
            unsafe { sys::debug_message(Ptr32::from_slice(bytes), bytes.len() as u32) };
        if !matches!(ret_code.into(), Err(Error::LoggingDisabled)) {
            // SAFETY: safe because executing in a single threaded context
            unsafe { DEBUG_ENABLED = true }
        }
        // SAFETY: safe because executing in a single threaded context
        unsafe { FIRST_RUN = false }
    }
}

#[cfg(not(feature = "ink-debug"))]
/// A no-op. Enable the `ink-debug` feature for debug messages.
pub fn debug_message(_message: &str) {}

macro_rules! impl_hash_fn {
    ( $name:ident, $bytes_result:literal ) => {
        paste::item! {
            pub fn [<hash_ $name>](input: &[u8], output: &mut [u8; $bytes_result]) {
                unsafe {
                    sys::[<hash_ $name>](
                        Ptr32::from_slice(input),
                        input.len() as u32,
                        Ptr32Mut::from_slice(output),
                    )
                }
            }
        }
    };
}
impl_hash_fn!(sha2_256, 32);
impl_hash_fn!(keccak_256, 32);
impl_hash_fn!(blake2_256, 32);
impl_hash_fn!(blake2_128, 16);

pub fn ecdsa_recover(
    signature: &[u8; 65],
    message_hash: &[u8; 32],
    output: &mut [u8; 33],
) -> Result {
    let ret_code = unsafe {
        sys::ecdsa_recover(
            Ptr32::from_slice(signature),
            Ptr32::from_slice(message_hash),
            Ptr32Mut::from_slice(output),
        )
    };
    ret_code.into()
}

pub fn ecdsa_to_eth_address(pubkey: &[u8; 33], output: &mut [u8; 20]) -> Result {
    let ret_code = unsafe {
        sys::ecdsa_to_eth_address(Ptr32::from_slice(pubkey), Ptr32Mut::from_slice(output))
    };
    ret_code.into()
}

pub fn is_contract(account_id: &[u8]) -> bool {
    let ret_val = unsafe { sys::is_contract(Ptr32::from_slice(account_id)) };
    ret_val.into_bool()
}

pub fn caller_is_origin() -> bool {
    let ret_val = unsafe { sys::caller_is_origin() };
    ret_val.into_bool()
}

pub fn set_code_hash(code_hash: &[u8]) -> Result {
    let ret_val = unsafe { sys::set_code_hash(Ptr32::from_slice(code_hash)) };
    ret_val.into()
}

pub fn code_hash(account_id: &[u8], output: &mut [u8]) -> Result {
    let mut output_len = output.len() as u32;
    let ret_val = unsafe {
        sys::code_hash(
            Ptr32::from_slice(account_id),
            Ptr32Mut::from_slice(output),
            Ptr32Mut::from_ref(&mut output_len),
        )
    };
    ret_val.into()
}

pub fn own_code_hash(output: &mut [u8]) {
    let mut output_len = output.len() as u32;
    unsafe {
        sys::own_code_hash(
            Ptr32Mut::from_slice(output),
            Ptr32Mut::from_ref(&mut output_len),
        )
    }
}
