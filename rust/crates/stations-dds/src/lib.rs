//! The `stations-dds` library.
//!
//! Provides DDS participant creation, typed readers and writers, and contract
//! validation for the to-stations prototype.

pub use cyclonedds;

/// The DDS type name that `cyclonedds` registers for `T`.
///
/// Re-exports the binding's [`cyclonedds::DdsType::type_name`] so callers and
/// contract validation can read it without naming the binding crate directly.
pub fn type_name<T: cyclonedds::DdsType>() -> &'static str {
    T::type_name()
}
