//! [`alloc::rc::Rc`] and [`alloc::sync::Arc`] alternatives that allow the
//! smart pointer to refer to just a portion of a reference-counted allocation.
//!
//! This allows storing a pointer that comes from a reference-counted allocation
//! without having to know the type of the allocation it came from. For example,
//! if you've allocated storage for an entire struct then you can derive a
//! pointer to just one of its fields while still keeping the entire allocation
//! live.
#![no_std]
#![cfg_attr(feature = "experimental_allocator_api", feature(allocator_api))]
#![cfg_attr(docsrs, feature(doc_cfg))]

extern crate alloc;

mod macroed;

#[cfg(feature = "arc")]
pub mod arc;
#[cfg(feature = "rc")]
pub mod rc;
