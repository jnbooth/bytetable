//! [`ByteTable`] and associated types.
#![allow(clippy::cast_possible_truncation)]

use core::array::{self, TryFromSliceError};
use core::borrow::{Borrow, BorrowMut};
use core::mem::MaybeUninit;
use core::ops::{
    Bound, Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo,
    RangeToInclusive,
};
pub use core::slice::{Iter, IterMut};

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

/// A lookup table where every possible `u8` (i.e. `0..=255`) is associated with
/// a value. As such, it may be safely indexed by `u8` without bounds checking.
/// The primary method of constructing a `ByteTable` is with
/// [`ByteTable::generate`], which accepts a function of `Fn(u8) -> T`. See its
/// documentation for more.
///
/// The table is backed by a 256-length array (`[T; 256]`). It does not perform
/// any allocation and even implements [`Copy`] if `T` does. Conversely, that
/// means its size is always `size_of<T>() * 256`. If stack size is an issue,
/// [`ByteTable::generate_boxed`] can be used to create the table directly on
/// the heap.
///
/// # Examples
///
/// ```
/// use bytetable::ByteTable;
///
/// let table = ByteTable::generate(|n| (n as usize).pow(3));
/// assert_eq!(table[99], 99usize.pow(3));
/// ```
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteTable<T> {
    table: [T; 256],
}

impl<T: Default> Default for ByteTable<T> {
    /// Creates a `ByteTable` where every value is `T::default()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteTable;
    ///
    /// assert_eq!(ByteTable::default(), ByteTable::new([""; 256]));
    /// ```
    #[inline]
    fn default() -> Self {
        Self::generate(|_| T::default())
    }
}

impl<T> ByteTable<T> {
    /// Creates a new `ByteTable` using the provided array for its contents.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteTable;
    ///
    /// let mut array = [15; 256];
    /// array[3] = 10;
    /// let table = ByteTable::new(array);
    /// assert_eq!(table[255], 15);
    /// assert_eq!(table[3], 10);
    /// ```
    #[inline]
    pub const fn new(table: [T; 256]) -> Self {
        Self { table }
    }

    /// Deconstructs the `ByteTable` into its inner array.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteTable;
    ///
    /// let table = ByteTable::new([15; 256]);
    /// assert_eq!(table.into_array(), [15; 256]);
    /// ```
    #[inline]
    pub fn into_array(self) -> [T; 256] {
        self.table
    }

    /// Creates a new `ByteTable` on the stack using the provided function to
    /// generate a value for every `u8`, i.e. `0..=255`.
    ///
    /// This function can be used to create a lookup table for a potentially
    /// expensive operation on `u8`s. Ordinarily, one would create an array
    /// with a default value instead, e.g. `[0; 256]`. However, that approach
    /// is not possible if `T` is neither const-initializable nor Copy-able.
    /// This function uses [`MaybeUninit`]s to obviate that constraint.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteTable;
    ///
    /// let table = ByteTable::generate(|n| (n as usize).pow(3));
    /// assert_eq!(table[99], 99usize.pow(3));
    /// ```
    #[inline]
    pub fn generate<F>(generator: F) -> Self
    where
        F: Fn(u8) -> T,
    {
        let mut table: [MaybeUninit<T>; 256] = [const { MaybeUninit::uninit() }; 256];
        for (i, elem) in table.iter_mut().enumerate() {
            elem.write(generator(i as u8));
        }
        Self {
            // SAFETY: `table` is fully initialized, so it should now be ready to be transmuted
            // from `[MaybeUninit<T>; 256]` to `[T; 256]`. Unfortunately, `mem::transmute` gets
            // confused by arrays, so we have to resort to pointer casting instead. Afterward,
            // there's no need for `mem::forget` because `MaybeUninit` obviates drop logic.
            //
            // See https://github.com/rust-lang/rust/issues/47966#issuecomment-606905342.
            table: unsafe { table.as_ptr().cast::<[T; 256]>().read() },
        }
    }

    /// Like [`ByteTable::generate`], but creates the table on the heap instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteTable;
    ///
    /// let table = ByteTable::generate_boxed(|n| (n as usize).pow(3));
    /// assert_eq!(table[99], 99usize.pow(3));
    /// ```
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn generate_boxed<F>(generator: F) -> Box<Self>
    where
        F: Fn(u8) -> T,
    {
        // SAFETY: By definition, `MaybeUninit`s do not require initialization.
        let mut table: Box<[MaybeUninit<T>; 256]> = unsafe { Box::new_uninit().assume_init() };
        for (i, elem) in table.iter_mut().enumerate() {
            elem.write(generator(i as u8));
        }
        let raw = Box::into_raw(table);
        // SAFETY:
        // 1. At this point, all elements in `table` have been fully initialized.
        //    `raw` may be safely interpreted as a pointer to `[T; 256]`.
        // 2. `ByteTable<T>` is `[repr(transparent)]` for `[T; 256]`.
        //    `raw` may be safely interpreted as a pointer to `ByteTable<T>`.
        // 3. `raw` was produced by `Box::into_raw`.
        //    It may safely be passed to `Box::from_raw`.
        unsafe { Box::from_raw(raw.cast()) }
    }

    /// Returns a reference to the item at the specified byte index.
    /// This function is indentical to an indexing operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteTable;
    ///
    /// let table = ByteTable::generate(|n| (n as usize) * 10);
    /// assert_eq!(*table.get(30), 300);
    /// assert_eq!(table[30], 300); // Same effect.
    /// ```
    #[inline]
    pub fn get(&self, i: u8) -> &T {
        // SAFETY: any u8 (0..256) is a valid index for [T; 256].
        unsafe { self.table.get_unchecked(i as usize) }
    }

    /// Returns a mutable reference to the item at the specified byte index.
    /// This function is indentical to an indexing operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteTable;
    ///
    /// let mut table = ByteTable::generate(|n| (n as usize) * 10);
    /// *table.get_mut(30) = 0;
    /// assert_eq!(table[30], 0);
    /// table[30] = 0; // Same effect.
    #[inline]
    pub fn get_mut(&mut self, i: u8) -> &mut T {
        // SAFETY: any u8 (0..256) is a valid index for [T; 256].
        unsafe { self.table.get_unchecked_mut(i as usize) }
    }

    /// Returns a `ByteTable` with the function `f` applied to each element in
    /// order.
    ///
    /// See [`array::map`].
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteTable;
    ///
    /// let times_10 = ByteTable::generate(|n| (n as usize) * 10);
    /// let plus_1 = times_10.map(|n| n + 1);
    /// assert_eq!(plus_1[5], 51);
    /// ```
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

    /// Borrows each element and returns a table of references.
    ///
    /// See [`array::each_ref`].
    #[inline]
    pub const fn each_ref(&self) -> ByteTable<&T> {
        ByteTable {
            table: self.table.each_ref(),
        }
    }

    /// Borrows each element mutably and returns a table of mutable references.
    ///
    /// See [`array::each_mut`].
    #[inline]
    pub const fn each_mut(&mut self) -> ByteTable<&mut T> {
        ByteTable {
            table: self.table.each_mut(),
        }
    }

    /// Returns a slice containing the entire table.
    /// Equivalent to `&s[..]`.
    #[inline]
    pub const fn as_slice(&self) -> &[T] {
        &self.table
    }

    /// Returns a mutable slice containing the entire table.
    /// Equivalent to `&mut s[..]`.
    #[inline]
    pub const fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.table
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

pub type IntoIter<T> = array::IntoIter<T, 256>;

impl<T> IntoIterator for ByteTable<T> {
    type Item = T;

    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.table.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a ByteTable<T> {
    type Item = &'a T;

    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.table.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut ByteTable<T> {
    type Item = &'a mut T;

    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.table.iter_mut()
    }
}

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

    #[inline]
    fn index(&self, index: RangeFull) -> &Self::Output {
        // SAFETY: `RangeFull` is always a valid index.
        unsafe { self.table.get_unchecked(index) }
    }
}
impl<T> IndexMut<RangeFull> for ByteTable<T> {
    #[inline]
    fn index_mut(&mut self, index: RangeFull) -> &mut Self::Output {
        // SAFETY: `RangeFull` is always a valid index.
        unsafe { self.table.get_unchecked_mut(index) }
    }
}

/// # Safety
///
/// For any `index: $idx` value, `$f(index)` must produce a slice index which is
/// in-bounds for a 256-length array. That is,
/// `[T; 256].get_unchecked($f(index))` must be safe.
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
    let exhausted = empty && start <= end;
    let exclusive_end = end as usize + 1;
    let start = if exhausted {
        exclusive_end
    } else {
        start as usize
    };
    start..exclusive_end
}
// SAFETY: Converted lower bound cannot exceed 255. Converted upper bound is at
// most 256. Since the `RangeInclusive` is converted to a `Range`, the upper
// bound is an exclusive bound. An exclusive range ending at 256 is safe to
// index into a 256-length array.
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
