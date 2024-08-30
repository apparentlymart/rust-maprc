extern crate alloc;

#[cfg(feature = "rc")]
use alloc::rc::Rc as AllocRc;
#[cfg(feature = "rc")]
use alloc::rc::Weak as AllocRcWeak;
#[cfg(feature = "arc")]
use alloc::sync::Arc as AllocArc;
#[cfg(feature = "arc")]
use alloc::sync::Weak as AllocArcWeak;

#[allow(unused)]
macro_rules! rc_wrapper {
    ($modname: ident, $strongname:ident, $weakname:ident, $basestrong:ident, $baseweak:ident) => {
        pub mod $modname {
            use super::$basestrong;
            use super::$baseweak;
            use super::Erased;
            #[allow(unused)]
            use core::mem::MaybeUninit;

            #[doc = concat!("Smart pointer to all or part of a reference-counted heap allocation.

This behaves the same as [`", stringify!($basestrong), "`] but has a different internal
structure that separates the reference-counted allocation from the pointer into
it, and thus the pointer can potentially refer to only part of the allocation,
such as a single field of an allocated struct.

However, that new capability makes this `Rc` three times larger than the standard
library implementation: it stores a pointer to the allocation, a pointer to
metadata about the allocation (to allow it to be dropped), and a pointer to
the relevant `T` inside that allocation. Storing the metadata in particular
allows erasing the static type of the allocation so that `", stringify!($strongname) ,"` is generic only
over what it points at regardless of what it's contained within.
")]
            pub struct $strongname<T: ?Sized> {
                ptr: *const T,
                owner: Option<$basestrong<dyn Erased>>,
            }

            impl<T: 'static> $strongname<T> {
                /// Creates a new reference-counted allocation containing the given value.
                #[inline(always)]
                pub fn new(v: T) -> Self {
                    let owner = $basestrong::new(v);
                    let ptr = $basestrong::as_ptr(&owner);
                    Self { ptr, owner: Some(owner) }
                }

                /// Constructs a new reference-counted allocation that could contain a weak pointer to itself.
                ///
                /// The given closure is passed a non-upgradeable weak reference to an allocation big
                /// enough to contain a `T`. After the closure returns its result is written into that
                /// allocation. The result may contain zero or more clones of the weak reference, which
                /// then become valid once `new_cyclic` returns.
                pub fn new_cyclic(data_fn: impl FnOnce(&$weakname<T>) -> T) -> Self {
                    let owner = $basestrong::new_cyclic(|alloc_weak| {
                        let weak = $weakname::from_alloc($baseweak::clone(alloc_weak));
                        data_fn(&weak)
                    });
                    let ptr = $basestrong::as_ptr(&owner);
                    Self { ptr, owner: Some(owner) }
                }

                #[doc = concat!("Transforms an [`", stringify!($basestrong) ,"`] into an [`", stringify!($strongname), "`] referring to the same allocation.")]
                #[inline(always)]
                pub fn from_alloc(v: $basestrong<T>) -> Self {
                    // TODO: It would be better if this were implemented for ?Sized
                    // but we can't type-erase a dynamically-sized T.
                    Self {
                        ptr: $basestrong::as_ptr(&v),
                        owner: Some(v as $basestrong<dyn Erased>),
                    }
                }
            }

            #[cfg(feature = "experimental_allocator_api")]
            #[cfg_attr(docsrs, doc(cfg(feature = "experimental_allocator_api")))]
            /// Additional functions that are available only with feature `experimental_allocator_api`,
            /// which in turn depends on the Rust experimental feature `allocator_api` and thus
            /// requires a nightly build and is subject to break in future.
            impl<T: 'static> $strongname<T> {
                /// Creates a new reference-counted allocation containing the given value,
                /// returning an error if the allocation fails.
                #[inline(always)]
                pub fn try_new(v: T) -> Result<Self, alloc::alloc::AllocError> {
                    let owner = $basestrong::try_new(v)?;
                    let ptr = $basestrong::as_ptr(&owner);
                    Ok(Self { ptr, owner: Some(owner) })
                }

                /// Creates a new reference-counted allocation suitable for `T` without
                /// initializing it, returning an error if the allocation fails.
                #[inline(always)]
                pub fn try_new_uninit() -> Result<$strongname<MaybeUninit<T>>, alloc::alloc::AllocError> {
                    let owner = $basestrong::try_new_uninit()?;
                    let ptr = $basestrong::as_ptr(&owner);
                    Ok($strongname { ptr, owner: Some(owner) })
                }
            }

            impl<T: ?Sized> $strongname<T> {
                #[doc = concat!("Wraps the target of the given static reference in an [`", stringify!($strongname) ,"`] that does not actually perform any reference counting and performs no heap allocation, because a static object is never dropped.\n\nThis is here to allow both reference-counted and statically-allocated objects to be stored in the same type where that's convenient, but wastes two pointers of storage compared to just using the reference directly.")]
                #[inline(always)]
                pub const fn from_static_ref(r: &'static T) -> Self {
                    Self {
                        ptr: r as *const _,
                        owner: None,
                    }
                }

                /// Gets a raw pointer to the target.
                ///
                /// The counts are not affected in any way and the pointer remains valid
                /// for as long as at least one strong reference remains live.
                #[inline(always)]
                pub const fn as_ptr(this: &Self) -> *const T {
                    this.ptr
                }

                /// Creates a new pointer to the same object.
                ///
                /// This increments the reference count for the underlying allocation.
                #[inline(always)]
                pub fn clone(this: &Self) -> Self {
                    Self {
                        ptr: this.ptr,
                        owner: this.owner.clone(),
                    }
                }

                /// Creates a new pointer to some part of the current pointer's target,
                /// within the same allocation.
                ///
                /// The closure receives a reference to the pointer's target and must
                /// return a reference with the same lifetime. The target of that new
                /// reference then becomes the target of the resulting pointer.
                pub fn clone_map<'a, R: ?Sized + 'a>(this: &'a Self, f: impl FnOnce(&'a T) -> &'a R) -> $strongname<R> {
                    let r = unsafe { &*this.ptr };
                    let r = f(r);
                    $strongname {
                        ptr: r as *const _,
                        owner: this.owner.clone(),
                    }
                }

                /// Conditionally creates a new pointer to some part of the current pointer's
                /// target, within the same allocation.
                ///
                /// The closure receives a reference to the pointer's target and may
                /// optionally return a reference with the same lifetime. If the closure
                /// returns `None` then no new pointer is created and so the strong reference
                /// count of the allocation remains unchanged.
                pub fn clone_filter_map<'a, R: ?Sized + 'a>(
                    this: &'a Self,
                    f: impl FnOnce(&'a T) -> Option<&'a R>,
                ) -> Option<$strongname<R>> {
                    let r = unsafe { &*this.ptr };
                    let maybe_r = f(r);
                    maybe_r.map(|r| $strongname {
                        ptr: r as *const _,
                        owner: this.owner.clone(),
                    })
                }

                /// Creates a weak pointer to the same target value.
                pub fn downgrade(this: &Self) -> Weak<T> {
                    match &this.owner {
                        Some(owner) => Weak {
                            ptr: this.ptr,
                            owner: Some($basestrong::downgrade(owner)),
                        },
                        None => Weak {
                            ptr: this.ptr,
                            owner: None,
                        }
                    }

                }

                /// Gets the number of strong pointers to this allocation.
                ///
                /// Returns [`usize::MAX`] if this reference was created using
                /// [`Self::from_static_ref`], because there is no true
                /// reference count for a static allocation.
                #[inline(always)]
                pub fn strong_count(this: &Self) -> usize {
                    match &this.owner {
                        Some(owner) => $basestrong::strong_count(owner),
                        None => usize::MAX,
                    }
                }

                /// Gets the number of weak pointers to this allocation.
                ///
                /// Returns [`usize::MAX`] if this reference was created using
                /// [`Self::from_static_ref`], because there is no true
                /// reference count for a static allocation.
                #[inline(always)]
                pub fn weak_count(this: &Self) -> usize {
                    match &this.owner {
                        Some(owner) => $basestrong::weak_count(owner),
                        None => usize::MAX,
                    }
                }

                /// Gets the size of the allocation containing the value this pointer refers to.
                ///
                /// This is _not_ the size of the pointee unless the pointer is to the whole
                /// allocation, as is true for the result of [`Self::new`].
                ///
                /// For a pointer into a static object created with [`Self::from_static_ref`],
                /// returns zero to represent that there is no dynamic allocation.
                #[inline(always)]
                pub fn allocation_size(this: &Self) -> usize {
                    match &this.owner {
                        Some(owner) => core::mem::size_of_val(core::ops::Deref::deref(owner)),
                        None => 0,
                    }
                }

                /// Returns `true` if `this` refers to a static object rather
                /// than to a reference-counted object.
                #[inline(always)]
                pub fn target_is_static(this: &Self) -> bool {
                    this.owner.is_none()
                }
            }

            impl<T: ?Sized> core::ops::Deref for $strongname<T> {
                type Target = T;

                /// Returns a reference to the pointee.
                #[inline(always)]
                fn deref(&self) -> &T {
                    unsafe { &*self.ptr }
                }
            }

            impl<T> core::convert::AsRef<T> for $strongname<T> {
                #[inline(always)]
                fn as_ref(&self) -> &T {
                    unsafe { &*self.ptr }
                }
            }

            impl<T> core::borrow::Borrow<T> for $strongname<T> {
                #[inline(always)]
                fn borrow(&self) -> &T {
                    unsafe { &*self.ptr }
                }
            }

            impl<T> core::clone::Clone for $strongname<T> {
                /// Creates a new pointer to the same value in the same allocation.
                ///
                /// This is equivalent to [`Self::clone`].
                #[inline(always)]
                fn clone(&self) -> Self {
                    $strongname::<T>::clone(self)
                }
            }

            impl<T: 'static> From<T> for $strongname<T> {
                /// Moves the value into a heap allocation and returns the first strong reference to it.
                ///
                /// Equivalent to [`Self::new`].
                #[inline(always)]
                fn from(value: T) -> Self {
                    Self::new(value)
                }
            }

            impl<T: 'static> From<$basestrong<T>> for $strongname<T> {
                /// Converts from the standard library implementation to this implementation while
                /// reusing the same underlying allocation.
                ///
                /// Equivalent to [`Self::from_alloc`].
                #[inline(always)]
                fn from(value: $basestrong<T>) -> Self {
                    Self::from_alloc(value)
                }
            }

            impl<T: 'static> From<alloc::boxed::Box<T>> for $strongname<T> {
                /// Converts from the standard library implementation to this implementation while
                /// reusing the same underlying allocation.
                ///
                /// Equivalent to [`Self::from_alloc`].
                fn from(value: alloc::boxed::Box<T>) -> Self {
                    let owner: $basestrong<T> = value.into();
                    Self::from_alloc(owner)
                }
            }

            impl<T: core::hash::Hash + ?Sized> core::hash::Hash for $strongname<T> {
                #[inline]
                fn hash<H>(&self, hasher: &mut H) where H: core::hash::Hasher {
                    use core::ops::Deref;
                    let r = self.deref();
                    <T as core::hash::Hash>::hash(r, hasher)
                }
            }

            impl<T: core::cmp::PartialEq + ?Sized> core::cmp::PartialEq for $strongname<T> {
                #[inline]
                fn eq(&self, other: &Self) -> bool {
                    use core::ops::Deref;
                    let r1 = self.deref();
                    let r2 = other.deref();
                    <T as core::cmp::PartialEq>::eq(r1, r2)
                }
            }

            impl<T: core::cmp::Eq + ?Sized> core::cmp::Eq for $strongname<T> {}

            impl<T: core::cmp::PartialOrd + ?Sized> core::cmp::PartialOrd for $strongname<T> {
                fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                    use core::ops::Deref;
                    let r1 = self.deref();
                    let r2 = other.deref();
                    <T as core::cmp::PartialOrd>::partial_cmp(r1, r2)
                }
            }

            impl<T: core::cmp::Ord + ?Sized> core::cmp::Ord for $strongname<T> {
                fn cmp(&self, other: &Self) -> core::cmp::Ordering {
                    use core::ops::Deref;
                    let r1 = self.deref();
                    let r2 = other.deref();
                    <T as core::cmp::Ord>::cmp(r1, r2)
                }
            }

            impl<T: core::default::Default + 'static> core::default::Default for $strongname<T> {
                #[inline(always)]
                fn default() -> Self {
                    Self::new(T::default())
                }
            }

            impl<T: ?Sized + core::fmt::Debug> core::fmt::Debug for $strongname<T> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    core::fmt::Debug::fmt(&**self, f)
                }
            }

            impl<T: ?Sized + core::fmt::Display> core::fmt::Display for $strongname<T> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    core::fmt::Display::fmt(&**self, f)
                }
            }

            impl<T: ?Sized + core::fmt::Pointer> core::fmt::Pointer for $strongname<T> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    core::fmt::Pointer::fmt(&**self, f)
                }
            }

            #[doc = concat!("Weak-reference counterpart of [`", stringify!($strongname), "`].")]
            pub struct $weakname<T: ?Sized> {
                ptr: *const T,
                owner: Option<$baseweak<dyn Erased>>,
            }

            impl<T: 'static> $weakname<T> {
                /// Constructs a new weak reference without performing a dynamic allocation.
                ///
                /// Calling [`Self::upgrade`] on the result always returns `None`.
                #[inline(always)]
                pub fn new() -> Self {
                    let owner = $baseweak::new();
                    Self {
                        ptr: $baseweak::as_ptr(&owner),
                        owner: Some(owner as $baseweak<dyn Erased>),
                    }
                }

                #[doc = concat!("Transforms an [`", stringify!($baseweak) ,"`] into an [`", stringify!($weakname), "`] referring to the same allocation.")]
                #[inline(always)]
                pub fn from_alloc(v: $baseweak<T>) -> Self {
                    Self {
                        ptr: $baseweak::as_ptr(&v),
                        owner: Some(v as $baseweak<dyn Erased>),
                    }
                }
            }

            impl<T: ?Sized> Weak<T> {
                #[doc = concat!("Wraps the target of the given static reference in a [`", stringify!($weakname) ,"`] that does not actually perform any reference counting and performs no heap allocation, because a static object is never dropped.\n\nThis is here to allow both reference-counted and statically-allocated objects to be stored in the same type where that's convenient, but wastes two pointers of storage compared to just using the reference directly.")]
                #[inline(always)]
                pub const fn from_static_ref(r: &'static T) -> Self {
                    Self {
                        ptr: r as *const _,
                        owner: None,
                    }
                }

                /// Attempts to upgrade the weak reference into a strong reference.
                ///
                /// Returns `None` if there are no strong references left live.
                #[inline(always)]
                pub fn upgrade(&self) -> Option<$strongname<T>> {
                    match &self.owner {
                        Some(owner) => owner.upgrade().map(
                            #[inline(always)]
                            |owner| $strongname {
                                ptr: self.ptr,
                                owner: Some(owner),
                            },
                        ),
                        None => Some($strongname {
                            ptr: self.ptr,
                            owner: None,
                        }),
                    }
                }

                /// Gets the number of strong pointers to this allocation.
                ///
                /// Returns [`usize::MAX`] if this reference was created using
                /// [`Self::from_static_ref`], because there is no true
                /// reference count for a static allocation.
                #[inline(always)]
                pub fn strong_count(&self) -> usize {
                    match &self.owner {
                        Some(owner) => owner.strong_count(),
                        None => usize::MAX,
                    }
                }

                /// Gets the number of weak pointers to this allocation.
                ///
                /// Returns [`usize::MAX`] if this reference was created using
                /// [`Self::from_static_ref`], because there is no true
                /// reference count for a static allocation.
                #[inline(always)]
                pub fn weak_count(&self) -> usize {
                    match &self.owner {
                        Some(owner) => owner.weak_count(),
                        None => usize::MAX,
                    }
                }

                /// Returns `true` if `self` refers to a static object rather
                /// than to a reference-counted object.
                ///
                /// A weak reference to a static object can always be upgraded
                /// because the static symbol acts as a permenent strong
                /// reference for the full duration of the program.
                #[inline(always)]
                pub fn target_is_static(&self) -> bool {
                    self.owner.is_none()
                }
            }

            impl<T: 'static> From<$baseweak<T>> for Weak<T> {
                #[inline(always)]
                fn from(value: $baseweak<T>) -> Self {
                    Self::from_alloc(value)
                }
            }

            impl<T: core::default::Default + 'static> core::default::Default for $weakname<T> {
                /// Returns a weak reference without any strong counterpart.
                ///
                /// Equivalent to [`Self::new`].
                #[inline(always)]
                fn default() -> Self {
                    Self::new()
                }
            }
        }
    };
}

#[cfg(feature = "rc")]
rc_wrapper!(rc, Rc, Weak, AllocRc, AllocRcWeak);
#[cfg(feature = "arc")]
rc_wrapper!(arc, Arc, Weak, AllocArc, AllocArcWeak);

/// An object-safe trait with no methods and thus whose trait objects
/// contain only an implementer's size and drop glue. The only requirement
/// is that the implementer not contain any non-static references, because
/// the smart pointer types would not be able to keep track of those
/// references.
#[allow(unused)]
trait Erased {}
impl<T: 'static> Erased for T {}
