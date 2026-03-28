//! Mutable manipulation of [`ByteString`]s for creating UTF-8 strings that can be cheaply cloned
//! and shared across threads without allocating memory.
//!
//! Internally, a `ByteStringMut` is a wrapper around a [`BytesMut`] buffer from the [`bytes`]
//! crate. It offers most of the same functionality, except it prevents the underlying data from
//! becoming invalid UTF-8. Due to this, it can safely be used to construct strings, as well as
//! dereferenced as a `&str` or `&mut str`.

#![no_std]

use core::array::{self, TryFromSliceError};
use core::borrow::{Borrow, BorrowMut};
use core::mem::MaybeUninit;
use core::ops::{
    Bound, Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo,
    RangeToInclusive,
};
use core::slice;

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteTable<T> {
    table: [T; 256],
}

impl<T: Default> Default for ByteTable<T> {
    #[inline]
    fn default() -> Self {
        Self::generate(|_| T::default())
    }
}

impl<T> ByteTable<T> {
    /// Creates a new `ByteTable` using the provided array for its contents.
    #[inline]
    pub const fn new(table: [T; 256]) -> Self {
        Self { table }
    }

    /// Deconstructs the `ByteTable` into its inner array.
    #[inline]
    pub fn into_array(self) -> [T; 256] {
        self.table
    }

    /// Creates a new `ByteTable` using the provided function to generate a value for every `u8`,
    /// i.e. `0..=255`.
    #[inline]
    pub fn generate<F>(generator: F) -> Self
    where
        F: Fn(u8) -> T,
    {
        let mut table: [MaybeUninit<T>; 256] = [const { MaybeUninit::uninit() }; 256];
        for (i, elem) in table.iter_mut().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            elem.write(generator(i as u8));
        }
        // SAFETY: `table` is fully initialized.
        unsafe { Self::assume_init(table) }
    }

    /// Returns a reference to the item at the specified byte index.
    /// This function never panics.
    #[inline]
    pub fn get(&self, i: u8) -> &T {
        unsafe { self.table.get_unchecked(i as usize) }
    }

    /// Returns a mutable reference to the item at the specified byte index.
    /// This function never panics.
    #[inline]
    pub fn get_mut(&mut self, i: u8) -> &mut T {
        unsafe { self.table.get_unchecked_mut(i as usize) }
    }

    /// Returns a `ByteTable` with the function f applied to each element in order.
    ///
    /// See [`array::map`].
    #[inline]
    #[must_use]
    pub fn map<F, U>(self, f: F) -> ByteTable<U>
    where
        F: FnMut(T) -> U,
    {
        ByteTable {
            table: self.table.map(f),
        }
    }

    /// Returns a slice containing the entire table. Equivalent to `&s[..]`.
    #[inline]
    pub const fn as_slice(&self) -> &[T] {
        &self.table
    }

    /// Returns a mutable slice containing the entire table. Equivalent to `&mut s[..]`.
    #[inline]
    pub const fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.table
    }

    /// Borrows each element and returns a table of references.
    ///
    /// See [`array::each_ref`].
    #[inline]
    pub const fn each_ref(&self) -> ByteTable<&T> {
        ByteTable {
            table: self.table.each_ref(),
        }
    }

    /// Borrows each element mutable and returns a table of mutable references.
    ///
    /// See [`array::each_mut`].
    #[inline]
    pub const fn each_mut(&mut self) -> ByteTable<&mut T> {
        ByteTable {
            table: self.table.each_mut(),
        }
    }

    /// Returns an array reference containing the entire table.
    #[inline]
    pub const fn as_array(&self) -> &[T; 256] {
        &self.table
    }

    /// Returns a mutable array reference containing the entire table.
    #[inline]
    pub const fn as_array_mut(&mut self) -> &mut [T; 256] {
        &mut self.table
    }

    /// # Safety
    ///
    /// `table` must be fully initialized.
    #[allow(clippy::needless_pass_by_value)]
    #[inline]
    #[must_use]
    const unsafe fn assume_init(table: [MaybeUninit<T>; 256]) -> Self {
        Self {
            table: unsafe { table.as_ptr().cast::<[T; 256]>().read() },
        }
    }
}

impl<T> Deref for ByteTable<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.table
    }
}
impl<T> DerefMut for ByteTable<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.table
    }
}

impl<T> AsRef<[T]> for ByteTable<T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}
impl<T> AsMut<[T]> for ByteTable<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T> Borrow<[T]> for ByteTable<T> {
    #[inline]
    fn borrow(&self) -> &[T] {
        self.as_slice()
    }
}
impl<T> BorrowMut<[T]> for ByteTable<T> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T> From<[T; 256]> for ByteTable<T> {
    #[inline]
    fn from(value: [T; 256]) -> Self {
        Self::new(value)
    }
}

impl<T> From<ByteTable<T>> for [T; 256] {
    #[inline]
    fn from(value: ByteTable<T>) -> Self {
        value.table
    }
}

impl<T: Copy> TryFrom<&[T]> for ByteTable<T> {
    type Error = TryFromSliceError;

    #[inline]
    fn try_from(value: &[T]) -> Result<Self, Self::Error> {
        value.try_into().map(Self::new)
    }
}

impl<T: Copy> TryFrom<&mut [T]> for ByteTable<T> {
    type Error = TryFromSliceError;

    #[inline]
    fn try_from(value: &mut [T]) -> Result<Self, Self::Error> {
        Self::try_from(&*value)
    }
}

impl<T> IntoIterator for ByteTable<T> {
    type Item = T;

    type IntoIter = array::IntoIter<T, 256>;

    fn into_iter(self) -> Self::IntoIter {
        self.table.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a ByteTable<T> {
    type Item = &'a T;

    type IntoIter = slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.table.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut ByteTable<T> {
    type Item = &'a mut T;

    type IntoIter = slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.table.iter_mut()
    }
}

impl<T> Index<u8> for ByteTable<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: u8) -> &Self::Output {
        self.get(index)
    }
}
impl<T> IndexMut<u8> for ByteTable<T> {
    #[inline]
    fn index_mut(&mut self, index: u8) -> &mut Self::Output {
        self.get_mut(index)
    }
}

impl<T> Index<RangeFull> for ByteTable<T> {
    type Output = [T];

    fn index(&self, index: RangeFull) -> &Self::Output {
        // SAFETY: `RangeFull` is always a valid index.
        unsafe { self.table.get_unchecked(index) }
    }
}
impl<T> IndexMut<RangeFull> for ByteTable<T> {
    fn index_mut(&mut self, index: RangeFull) -> &mut Self::Output {
        // SAFETY: `RangeFull` is always a valid index.
        unsafe { self.table.get_unchecked_mut(index) }
    }
}

/// # Safety
///
/// For any `index: $idx` value, `$f(index)` must produce a slice index which is in-bounds for a
/// 256-length array. That is, `[T; 256].get_unchecked($f(index))` must be safe.
macro_rules! unsafe_impl_index {
    ($idx:ty, $f:ident) => {
        impl<T> Index<$idx> for ByteTable<T> {
            type Output = [T];

            #[inline]
            fn index(&self, index: $idx) -> &Self::Output {
                let index = $f(index);
                unsafe { self.table.get_unchecked(index) }
            }
        }
        impl<T> IndexMut<$idx> for ByteTable<T> {
            #[inline]
            fn index_mut(&mut self, index: $idx) -> &mut Self::Output {
                let index = $f(index);
                unsafe { self.table.get_unchecked_mut(index) }
            }
        }
    };
}

#[inline(always)]
const fn convert_range(range: Range<u8>) -> Range<usize> {
    range.start as usize..range.end as usize
}
// SAFETY: Converted bounds cannot exceed 255.
unsafe_impl_index!(Range<u8>, convert_range);

#[inline(always)]
const fn convert_range_from(range: RangeFrom<u8>) -> RangeFrom<usize> {
    range.start as usize..
}
// SAFETY: Converted bound cannot exceed 255.
unsafe_impl_index!(RangeFrom<u8>, convert_range_from);

#[inline(always)]
const fn convert_range_to(range: RangeTo<u8>) -> RangeTo<usize> {
    ..range.end as usize
}
// SAFETY: Converted bound cannot exceed 255.
unsafe_impl_index!(RangeTo<u8>, convert_range_to);

#[inline(always)]
const fn convert_range_to_inclusive(range: RangeToInclusive<u8>) -> RangeToInclusive<usize> {
    ..=range.end as usize
}
// SAFETY: Converted bound cannot exceed 255.
unsafe_impl_index!(RangeToInclusive<u8>, convert_range_to_inclusive);

#[inline]
fn convert_range_inclusive(range: RangeInclusive<u8>) -> Range<usize> {
    // Replicates `RangeInclusive::into_slice_range`.
    let empty = range.is_empty();
    let (start, end) = range.into_inner();
    let exhausted = empty && start == end;
    let exclusive_end = end as usize + 1;
    let start = if exhausted {
        exclusive_end
    } else {
        start as usize
    };
    start..exclusive_end
}
// SAFETY: Converted lower bound cannot exceed 255. Converted upper bound is at most 256.
// Since the RangeInclusive is converted to a Range, the upper bound is an exclusive bound.
// An exclusive range ending at 256 is safe to index into a 256-length array.
unsafe_impl_index!(RangeInclusive<u8>, convert_range_inclusive);

#[inline]
const fn convert_bounds(bounds: (Bound<u8>, Bound<u8>)) -> (Bound<usize>, Bound<usize>) {
    #[inline]
    const fn convert_bound(bound: Bound<u8>) -> Bound<usize> {
        match bound {
            Bound::Included(n) => Bound::Included(n as usize),
            Bound::Excluded(n) => Bound::Excluded(n as usize),
            Bound::Unbounded => Bound::Unbounded,
        }
    }
    (convert_bound(bounds.0), convert_bound(bounds.1))
}
// SAFETY: Converted bounds cannot exceed 255.
unsafe_impl_index!((Bound<u8>, Bound<u8>), convert_bounds);

macro_rules! impl_eq {
    ($t:ty, $($d:tt)*) => {
        impl<U, T: PartialEq<U>> PartialEq<$t> for ByteTable<T> {
            #[inline]
            fn eq(&self, other: &$t) -> bool {
                self.table == $($d)* other
            }
            #[inline]
            fn ne(&self, other: &$t) -> bool {
                self.table != $($d)* other
            }
        }

        impl<T, U: PartialEq<T>> PartialEq<ByteTable<T>> for $t {
            #[inline]
            fn eq(&self, other: &ByteTable<T>) -> bool {
                $($d)* self == other.table
            }
            #[inline]
            fn ne(&self, other: &ByteTable<T>) -> bool {
                $($d)* self != other.table
            }
        }
    };
}

impl_eq!([U; 256], *);
impl_eq!([U], *);
impl_eq!(&[U], **);
impl_eq!(&[U; 256], **);
impl_eq!(&mut [U], **);
impl_eq!(&mut [U; 256], **);
