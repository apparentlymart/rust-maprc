#![no_std]

use alloc::rc::Rc as AllocRc;
use alloc::rc::Weak as AllocRcWeak;

extern crate alloc;

/// Smart pointer to all or part of a reference-counted heap allocation.
///
/// This behaves the same as [`alloc::rc::Rc`] but has a different internal
/// structure that separates the reference-counted allocation from the pointer into
/// it, and thus the pointer can potentially refer to only part of the allocation,
/// such as a single field of an allocated struct.
///
/// However, that new capability makes this `Rc` three times larger than the standard
/// library implementation: it stores a pointer to the allocation, a pointer to
/// metadata about the allocation (to allow it to be dropped), and a pointer to
/// the relevant `T` inside that allocation. Storing the metadata in particular
/// allows erasing the type of the allocation so that an `Rc` is generic only over
/// what it points at, and not over the type of the allocation that contains it.
pub struct Rc<T: ?Sized> {
    ptr: *const T,
    owner: AllocRc<dyn Erased>,
}

impl<T: 'static> Rc<T> {
    #[inline(always)]
    pub fn new(v: T) -> Self {
        let owner = AllocRc::new(v);
        let ptr = AllocRc::as_ptr(&owner);
        Self { ptr, owner }
    }

    pub fn new_cyclic(data_fn: impl FnOnce(&Weak<T>) -> T) -> Self {
        let owner = AllocRc::new_cyclic(|alloc_weak| {
            let weak = Weak::from_alloc(AllocRcWeak::clone(alloc_weak));
            data_fn(&weak)
        });
        let ptr = AllocRc::as_ptr(&owner);
        Self { ptr, owner }
    }

    /// Transform an [`alloc::rc::Rc`] into an [`Rc`] referring to the
    /// same allocation.
    #[inline(always)]
    pub fn from_alloc(v: AllocRc<T>) -> Self {
        Self {
            ptr: AllocRc::as_ptr(&v),
            owner: v as AllocRc<dyn Erased>,
        }
    }
}

impl<T: ?Sized> Rc<T> {
    #[inline(always)]
    pub const fn as_ptr(this: &Self) -> *const T {
        this.ptr
    }

    pub fn clone_map<'a, R: ?Sized + 'a>(this: &'a Self, f: impl FnOnce(&'a T) -> &'a R) -> Rc<R> {
        let r = unsafe { &*this.ptr };
        let r = f(r);
        Rc {
            ptr: r as *const _,
            owner: this.owner.clone(),
        }
    }

    pub fn clone_filter_map<'a, R: ?Sized + 'a>(
        this: &'a Self,
        f: impl FnOnce(&'a T) -> Option<&'a R>,
    ) -> Option<Rc<R>> {
        let r = unsafe { &*this.ptr };
        let maybe_r = f(r);
        maybe_r.map(|r| Rc {
            ptr: r as *const _,
            owner: this.owner.clone(),
        })
    }

    pub fn downgrade(this: &Self) -> Weak<T> {
        Weak {
            ptr: this.ptr,
            owner: AllocRc::downgrade(&this.owner),
        }
    }

    #[inline(always)]
    pub fn strong_count(this: &Self) -> usize {
        AllocRc::strong_count(&this.owner)
    }

    #[inline(always)]
    pub fn weak_count(this: &Self) -> usize {
        AllocRc::weak_count(&this.owner)
    }
}

impl<T: ?Sized> core::ops::Deref for Rc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.ptr }
    }
}

impl<T: 'static> From<T> for Rc<T> {
    #[inline(always)]
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: 'static> From<AllocRc<T>> for Rc<T> {
    #[inline(always)]
    fn from(value: AllocRc<T>) -> Self {
        Self::from_alloc(value)
    }
}

/// Weak-reference counterpart of [`Rc`].
pub struct Weak<T: ?Sized> {
    ptr: *const T,
    owner: AllocRcWeak<dyn Erased>,
}

impl<T: 'static> Weak<T> {
    #[inline(always)]
    pub fn from_alloc(v: AllocRcWeak<T>) -> Self {
        Self {
            ptr: AllocRcWeak::as_ptr(&v),
            owner: v as AllocRcWeak<dyn Erased>,
        }
    }
}

impl<T: ?Sized> Weak<T> {
    #[inline(always)]
    pub fn upgrade(&self) -> Option<Rc<T>> {
        self.owner.upgrade().map(
            #[inline(always)]
            |owner| Rc {
                ptr: self.ptr,
                owner,
            },
        )
    }

    #[inline(always)]
    pub fn strong_count(&self) -> usize {
        self.owner.strong_count()
    }

    #[inline(always)]
    pub fn weak_count(&self) -> usize {
        self.owner.weak_count()
    }
}

impl<T: 'static> From<AllocRcWeak<T>> for Weak<T> {
    #[inline(always)]
    fn from(value: AllocRcWeak<T>) -> Self {
        Self::from_alloc(value)
    }
}

/// An object-safe trait with no methods and thus whose trait objects
/// contain only an implementer's size and drop glue. The only requirement
/// is that the implementer not contain any non-static references, because
/// the smart pointer types would not be able to keep track of those
/// references.
trait Erased {}
impl<T: 'static> Erased for T {}
