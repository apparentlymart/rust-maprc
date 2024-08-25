//! Thread-safe reference-counting pointers. ‘Arc’ stands for ‘Atomically Reference Counted’.
//!
//! This module is only included when the "arc" feature is enabled, but that feature is enabled by default.

pub use crate::macroed::arc::*;

unsafe impl<T> Sync for Arc<T> {}
unsafe impl<T> Send for Arc<T> {}
unsafe impl<T> Sync for Weak<T> {}
unsafe impl<T> Send for Weak<T> {}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;

    #[test]
    pub fn new() {
        let p = Arc::new(24_u64);
        assert_eq!(*p, 24);
        assert_eq!(Arc::allocation_size(&p), 8, "wrong allocation size")
    }

    #[test]
    pub fn naked_into() {
        let p: Arc<u64> = 24.into();
        assert_eq!(*p, 24);
    }

    #[test]
    pub fn from_alloc() {
        let normal = alloc::sync::Arc::new(24);
        let p = Arc::from_alloc(normal);
        assert_eq!(*p, 24);
    }

    #[test]
    pub fn alloc_into() {
        let p: Arc<u64> = alloc::sync::Arc::new(24).into();
        assert_eq!(*p, 24);
    }

    #[test]
    pub fn clone_map() {
        struct Foo {
            a: u64,
            b: u64,
        }
        let foo = Arc::new(Foo { a: 3, b: 4 });
        let foo_a: Arc<u64> = Arc::clone_map(&foo, |foo| &foo.a);
        let foo_b: Arc<u64> = Arc::clone_map(&foo, |foo| &foo.b);
        drop(foo); // The two clones can safely outlive the original
        assert_eq!(
            Arc::strong_count(&foo_a),
            2,
            "pointer to a has wrong strong_count",
        );
        assert_eq!(
            Arc::strong_count(&foo_b),
            2,
            "pointer to b has wrong strong_count",
        );
        assert_eq!(*foo_a, 3);
        assert_eq!(*foo_b, 4);
    }

    #[test]
    pub fn clone_filter_map() {
        struct Foo {
            a: Option<u64>,
            b: Option<u64>,
        }
        let foo = Arc::new(Foo {
            a: Some(3),
            b: None,
        });
        let foo_a: Option<Arc<u64>> = Arc::clone_filter_map(&foo, |foo| foo.a.as_ref());
        let foo_b: Option<Arc<u64>> = Arc::clone_filter_map(&foo, |foo| foo.b.as_ref());
        assert_eq!(
            Arc::strong_count(&foo),
            2, // foo_b doesn't have a reference
            "pointer to foo has wrong strong_count",
        );
        assert_eq!(foo_a.map(|r| *r), Some(3));
        assert_eq!(foo_b.map(|r| *r), None);
    }
}
