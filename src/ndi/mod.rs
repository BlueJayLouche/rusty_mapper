//! # NDI Module
//!
//! Network Device Interface (NDI) video input and output support.
//!
//! ## Architecture
//! 
//! ### Input (`NdiReceiver`)
//! - Dedicated receiver thread for each NDI source
//! - Receives BGRA frames, converts to RGBA for GPU
//! - Bounded channel with latest-frame-only semantics
//! 
//! ### Output (`NdiOutputSender`)
//! - Dedicated sender thread to avoid blocking render loop
//! - Receives BGRA frames (native format, no conversion needed)
//! - Bounded channel (capacity=2) for low latency

pub mod output;
pub use output::{NdiOutputSender, is_ndi_output_available};
