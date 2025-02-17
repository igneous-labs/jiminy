//! Copied from
//! https://github.com/anza-xyz/solana-sdk/blob/master/define-syscall/src/lib.rs
//!
//! with
//! - `no_std` slapped atop
//! - `static-syscalls` feature disabled, because that doesnt seem to work yet?

#![cfg_attr(not(test), no_std)]

mod codes;
mod definitions;

pub use codes::*;
pub use definitions::*;

macro_rules! define_syscall {
    (fn $name:ident($($arg:ident: $typ:ty),*) -> $ret:ty) => {
        extern "C" {
            pub fn $name($($arg: $typ),*) -> $ret;
        }
    };
    (fn $name:ident($($arg:ident: $typ:ty),*)) => {
        define_syscall!(fn $name($($arg: $typ),*) -> ());
    }
}

pub(crate) use define_syscall;
