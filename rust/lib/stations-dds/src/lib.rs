//! The `stations-dds` library.
//!
//! Provides DDS participant creation, typed readers and writers, and contract
//! validation for the to-stations prototype.

pub mod contract;
pub mod dds;

pub use cyclonedds;

/// The DDS type name that `cyclonedds` registers for `T`.
///
/// Re-exports the binding's [`cyclonedds::DdsType::type_name`] so callers and
/// contract validation can read it without naming the binding crate directly.
pub fn type_name<T: cyclonedds::DdsType>() -> &'static str {
    T::type_name()
}

include!(concat!(env!("REPO_ROOT"), "/build/gen/rust/hello_world.rs"));

#[cfg(test)]
mod tests {
    use super::{hello_world, math};
    use ::cyclonedds::DdsType;

    #[test]
    fn type_names_match_scopes() {
        assert_eq!(<math::Quaternion as DdsType>::type_name(), "math::Quaternion");
        assert_eq!(
            <hello_world::HelloWorldTopic as DdsType>::type_name(),
            "hello_world::HelloWorldTopic"
        );
    }
}
