//! The correction decision, with no knowledge of any operating system.
//!
//! Keeping this crate platform-free is what lets the hard part — deciding
//! *whether* to rewrite a user's word — be built and measured on one machine
//! and shipped to every platform unchanged.

pub mod buffer;
pub mod decide;

pub use buffer::{Break, Buffer};
pub use decide::{Correction, Decision, Engine, Thresholds};
