//! Single-threaded reference-counting pointers. ‘Rc’ stands for ‘Reference Counted’.
//!
//! This module is only included when the "rc" feature is enabled, but that feature is enabled by default.

pub use crate::macroed::rc::*;

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;

    #[test]
    pub fn new() {
        let p = Rc::new(24_u64);
        assert_eq!(*p, 24);
        assert_eq!(Rc::allocation_size(&p), 8, "wrong allocation size")
    }

    #[test]
    pub fn naked_into() {
        let p: Rc<u64> = 24.into();
        assert_eq!(*p, 24);
    }

    #[test]
    pub fn from_alloc() {
        let normal = alloc::rc::Rc::new(24);
        let p = Rc::from_alloc(normal);
        assert_eq!(*p, 24);
    }

    #[test]
    pub fn alloc_into() {
        let p: Rc<u64> = alloc::rc::Rc::new(24).into();
        assert_eq!(*p, 24);
    }

    #[test]
    pub fn clone_map() {
        struct Foo {
            a: u64,
            b: u64,
        }
        let foo = Rc::new(Foo { a: 3, b: 4 });
        let foo_a: Rc<u64> = Rc::clone_map(&foo, |foo| &foo.a);
        let foo_b: Rc<u64> = Rc::clone_map(&foo, |foo| &foo.b);
        drop(foo); // The two clones can safely outlive the original
        assert_eq!(
            Rc::strong_count(&foo_a),
            2,
            "pointer to a has wrong strong_count",
        );
        assert_eq!(
            Rc::strong_count(&foo_b),
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
        let foo = Rc::new(Foo {
            a: Some(3),
            b: None,
        });
        let foo_a: Option<Rc<u64>> = Rc::clone_filter_map(&foo, |foo| foo.a.as_ref());
        let foo_b: Option<Rc<u64>> = Rc::clone_filter_map(&foo, |foo| foo.b.as_ref());
        assert_eq!(
            Rc::strong_count(&foo),
            2, // foo_b doesn't have a reference
            "pointer to foo has wrong strong_count",
        );
        assert_eq!(foo_a.map(|r| *r), Some(3));
        assert_eq!(foo_b.map(|r| *r), None);
    }
}
