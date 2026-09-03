//! Private experimental C boundary used to pressure AR-0017.
//!
//! Nothing in this crate is a stable ABI or supported mobile surface.

#![deny(unsafe_code)]

mod boundary;
#[allow(unsafe_code)]
mod raw;

pub use raw::TosumuExperimentalV1Outcome;
