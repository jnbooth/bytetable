//! [`ByteSet`] and associated types.

use core::fmt;
use core::hint::unreachable_unchecked;
use core::iter::FusedIterator;
use core::ops::{
    BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Bound, Index, Not, RangeBounds,
    RangeFull, RangeInclusive,
};

/// A set of `u8`s, implemented as a bit array.
///
/// # Examples
///
/// ```
/// use bytetable::ByteSet;
///
/// let mut set = ByteSet::new() | (18..28) | (6..=10);
/// set.insert(100);
/// assert!(set.contains(20));
/// set.remove(20);
/// assert!(!set.contains(20));
/// assert_eq!(set.min(), Some(6));
/// assert_eq!(set.max(), Some(100));
/// let els = set.into_iter().collect::<Vec<u8>>();
/// assert_eq!(els, [6, 7, 8, 9, 10, 18, 19, 21, 22, 23, 24, 25, 26, 27, 100]);
/// assert_eq!(set.len(), 15);
/// ```
#[repr(transparent)]
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct ByteSet {
    bytes: [u64; 4],
}

impl ByteSet {
    /// Creates an empty `ByteSet`.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let set = ByteSet::new();
    /// assert_eq!(set.len(), 0);
    /// ```
    #[inline]
    pub const fn new() -> Self {
        Self { bytes: [0; 4] }
    }

    /// Creates a `ByteSet` containing every byte (`0..=255`).
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let set = ByteSet::full();
    /// assert_eq!(set.len(), 256);
    /// ```
    #[inline]
    pub const fn full() -> Self {
        Self {
            bytes: [u64::MAX; 4],
        }
    }

    /// Creates a `ByteSet` containing the bytes in the specified slice.
    /// This is intended for use in constant contexts. Otherwise,
    /// `ByteSet::from(slice)` accomplishes the same thing.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// const SET: ByteSet = ByteSet::from_bytes(&[10, 20, 30, 40]);
    /// assert_eq!(SET.len(), 4);
    /// let set = ByteSet::from(&[10, 20, 30, 40]); // Same effect.
    /// ```
    #[inline]
    pub const fn from_bytes(slice: &[u8]) -> Self {
        let mut set = Self::new();
        let mut i = 0;
        while i < slice.len() {
            set.insert(slice[i]);
            i += 1;
        }
        set
    }

    /// An iterator visiting all elements in ascending order.
    /// The iterator type is `u8`.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let set = ByteSet::from(10..15);
    /// let set_vec: Vec<u8> = set.iter().collect();
    /// assert_eq!(set_vec, [10, 11, 12, 13, 14]);
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        self.into_iter()
    }

    /// Returns the number of elements in the set.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let set = ByteSet::from(10..15);
    /// assert_eq!(set.len(), 5);
    /// ```
    #[inline]
    pub const fn len(&self) -> usize {
        let [a, b, c, d] = self.bytes;
        (a.count_ones() + b.count_ones() + c.count_ones() + d.count_ones()) as usize
    }

    /// Returns `true` if the set contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let mut set = ByteSet::new();
    /// assert!(set.is_empty());
    /// set.insert(1);
    /// assert!(!set.is_empty());
    /// ```
    #[inline]
    pub const fn is_empty(&self) -> bool {
        let [a, b, c, d] = self.bytes;
        a == 0 && b == 0 && c == 0 && d == 0
    }

    /// Creates an iterator which uses a closure to determine if an element
    /// should be removed.
    ///
    /// If the closure returns `true`, the element is removed from the set and
    /// yielded. If the closure returns `false`, or panics, the element remains
    /// in the set and will not be yielded.
    ///
    /// If the returned `ExtractIf` is not exhausted, e.g. because it is dropped
    /// without iterating or the iteration short-circuits, then the remaining
    /// elements will be retained.
    /// Use [`retain`] with a negated predicate if you do not need the returned
    /// iterator.
    ///
    /// [`retain`]: ByteSet::retain
    ///
    /// # Examples
    ///
    /// Splitting a set into even and odd values, reusing the original set:
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let mut set: ByteSet = (0..8).into();
    /// let extracted: ByteSet = set.extract_if(|v| v % 2 == 0).collect();
    ///
    /// let mut evens = extracted.into_iter().collect::<Vec<_>>();
    /// let mut odds = set.into_iter().collect::<Vec<_>>();
    /// evens.sort();
    /// odds.sort();
    ///
    /// assert_eq!(evens, vec![0, 2, 4, 6]);
    /// assert_eq!(odds, vec![1, 3, 5, 7]);
    /// ```
    #[must_use = "use `retain` with a negated predicate if you do not need the returned iterator"]
    #[inline]
    pub fn extract_if<F>(&mut self, pred: F) -> ExtractIf<'_, F>
    where
        F: FnMut(u8) -> bool,
    {
        ExtractIf {
            inner: self.range(),
            set: self,
            pred,
        }
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all elements `e` for which `f(e)` returns `false`.
    /// The elements are visited in ascending order.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let mut set = ByteSet::from(1..=6);
    /// set.retain(|k| k % 2 == 0);
    /// assert_eq!(set, [2, 4, 6].into());
    /// ```
    #[inline]
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(u8) -> bool,
    {
        for i in 0..=255 {
            if self.contains(i) && !f(i) {
                self.remove(i);
            }
        }
    }

    /// Clears the set, removing all values.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let mut v = ByteSet::new();
    /// v.insert(1);
    /// v.clear();
    /// assert!(v.is_empty());
    /// ```
    #[inline]
    pub const fn clear(&mut self) {
        self.bytes = [0; 4];
    }

    /// Returns a new `ByteSet` containing the difference,
    /// i.e., the values that are in `self` but not in `other`.
    /// Equivalent to [`self & !other`](ByteSet::not).
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let a = ByteSet::from([1, 2, 3]);
    /// let b = ByteSet::from([4, 2, 3, 4]);
    ///
    /// let diff = a.difference(b);
    /// assert_eq!(diff, [1].into());
    ///
    /// // Note that difference is not symmetric,
    /// // and `b - a` means something else:
    /// let diff = b.difference(a);
    /// assert_eq!(diff, [4].into());
    /// ```
    #[must_use]
    #[inline]
    pub const fn difference(self, other: Self) -> Self {
        let [a0, a1, a2, a3] = self.bytes;
        let [b0, b1, b2, b3] = other.bytes;
        Self {
            bytes: [a0 & !b0, a1 & !b1, a2 & !b2, a3 & !b3],
        }
    }

    /// Visits the values representing the symmetric difference,
    /// i.e., the values that are in `self` or in `other` but not in both.
    /// Equivalent to [`self ^ other`](ByteSet::bitxor).
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let a = ByteSet::from([1, 2, 3]);
    /// let b = ByteSet::from([4, 2, 3, 4]);
    ///
    /// let diff1 = a.symmetric_difference(b);
    /// let diff2 = b.symmetric_difference(a);
    ///
    /// assert_eq!(diff1, diff2);
    /// assert_eq!(diff1, [1, 4].into());
    /// ```
    #[must_use]
    #[inline]
    pub const fn symmetric_difference(self, other: Self) -> Self {
        let [a0, a1, a2, a3] = self.bytes;
        let [b0, b1, b2, b3] = other.bytes;
        Self {
            bytes: [a0 ^ b0, a1 ^ b1, a2 ^ b2, a3 ^ b3],
        }
    }

    /// Visits the values representing the intersection,
    /// i.e., the values that are both in `self` and `other`.
    /// Equivalent to [`self & other`](ByteSet::bitand).
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let a = ByteSet::from([1, 2, 3]);
    /// let b = ByteSet::from([4, 2, 3, 4]);
    ///
    /// let intersection = a.intersection(b);
    /// assert_eq!(intersection, [2, 3].into());
    /// ```
    #[must_use]
    #[inline]
    pub const fn intersection(self, other: Self) -> Self {
        let [a0, a1, a2, a3] = self.bytes;
        let [b0, b1, b2, b3] = other.bytes;
        Self {
            bytes: [a0 & b0, a1 & b1, a2 & b2, a3 & b3],
        }
    }

    /// Visits the values representing the union,
    /// i.e., all the values in `self` or `other`, without duplicates.
    /// Equivalent to [`self | other`](ByteSet::bitor).
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let a = ByteSet::from([1, 2, 3]);
    /// let b = ByteSet::from([4, 2, 3, 4]);
    ///
    /// let union = a.union(b);
    /// assert_eq!(union, [1, 2, 3, 4].into());
    /// ```
    #[must_use]
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        let [a0, a1, a2, a3] = self.bytes;
        let [b0, b1, b2, b3] = other.bytes;
        Self {
            bytes: [a0 | b0, a1 | b1, a2 | b2, a3 | b3],
        }
    }

    /// Returns `true` if the set contains a value.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let set = ByteSet::from([1, 2, 3]);
    /// assert_eq!(set.contains(1), true);
    /// assert_eq!(set.contains(4), false);
    /// ```
    #[inline]
    pub const fn contains(&self, i: u8) -> bool {
        let (high, low) = Self::indices(i);
        self.bytes[high] & low != 0
    }

    /// Returns `true` if `self` has no elements in common with `other`.
    /// Equivalent to [`(self & other).is_empty()`](ByteSet::bitand).
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let a = ByteSet::from([1, 2, 3]);
    /// let mut b = ByteSet::new();
    ///
    /// assert_eq!(a.is_disjoint(&b), true);
    /// b.insert(4);
    /// assert_eq!(a.is_disjoint(&b), true);
    /// b.insert(1);
    /// assert_eq!(a.is_disjoint(&b), false);
    /// ```
    #[inline]
    pub const fn is_disjoint(&self, other: &Self) -> bool {
        let [a0, a1, a2, a3] = self.bytes;
        let [b0, b1, b2, b3] = other.bytes;
        (a0 & b0) == 0 && (a1 & b1) == 0 && (a2 & b2) == 0 && (a3 & b3) == 0
    }

    /// Returns `true` if the set is a subset of another,
    /// i.e., `other` contains at least all the values in `self`.
    /// Equivalent to [`(self | other) == other`](ByteSet::bitor).
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let sup = ByteSet::from([1, 2, 3]);
    /// let mut set = ByteSet::new();
    ///
    /// assert_eq!(set.is_subset(&sup), true);
    /// set.insert(2);
    /// assert_eq!(set.is_subset(&sup), true);
    /// set.insert(4);
    /// assert_eq!(set.is_subset(&sup), false);
    /// ```
    #[inline]
    pub const fn is_subset(&self, other: &Self) -> bool {
        let [a0, a1, a2, a3] = self.bytes;
        let [b0, b1, b2, b3] = other.bytes;
        (a0 | b0) == b0 && (a1 | b1) == b1 && (a2 | b2) == b2 && (a3 | b3) == b3
    }

    /// Returns `true` if the set is a superset of another,
    /// i.e., `self` contains at least all the values in `other`.
    /// Equivalent to [`(other | self) == self`](ByteSet::bitor).
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let sub = ByteSet::from([1, 2]);
    /// let mut set = ByteSet::new();
    ///
    /// assert_eq!(set.is_superset(&sub), false);
    ///
    /// set.insert(0);
    /// set.insert(1);
    /// assert_eq!(set.is_superset(&sub), false);
    ///
    /// set.insert(2);
    /// assert_eq!(set.is_superset(&sub), true);
    /// ```
    #[inline]
    pub const fn is_superset(&self, other: &Self) -> bool {
        other.is_subset(self)
    }

    /// Returns `true` if `self` has no elements in common with `other`.
    /// Equivalent to [`self == other`](ByteSet::eq), but can be used
    /// in constant contexts.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// const A: ByteSet = ByteSet::from_bytes(&[5, 10, 15]);
    /// const B: ByteSet = ByteSet::from_bytes(&[5, 10, 15, 20]);
    /// const C: ByteSet = ByteSet::from_bytes(&[15, 10, 5]);
    /// const A_EQUALS_B: bool = A.const_eq(&B);
    /// const A_EQUALS_C: bool = A.const_eq(&C);
    ///
    /// assert!(!A_EQUALS_B);
    /// assert!(A_EQUALS_C);
    /// ```
    #[inline]
    pub const fn const_eq(&self, other: &Self) -> bool {
        let [a0, a1, a2, a3] = self.bytes;
        let [b0, b1, b2, b3] = other.bytes;
        a0 == b0 && a1 == b1 && a2 == b2 && a3 == b3
    }

    /// Adds a value to the set.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let mut set = ByteSet::new();
    ///
    /// set.insert(2);
    /// set.insert(2);
    /// assert_eq!(set.len(), 1);
    /// ```
    #[inline]
    pub const fn insert(&mut self, i: u8) {
        let (high, low) = Self::indices(i);
        self.bytes[high] |= low;
    }

    /// Removes a value from the set.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let mut set = ByteSet::new();
    ///
    /// set.insert(2);
    /// set.remove(2);
    /// assert_eq!(set.len(), 0);
    /// ```
    #[inline]
    pub const fn remove(&mut self, i: u8) {
        let (high, low) = Self::indices(i);
        self.bytes[high] &= !low;
    }

    /// Adds a value to the set if it is not present, or removes it from the
    /// set if it is present.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let mut set = ByteSet::new();
    ///
    /// set.toggle(2);
    /// assert!(set.contains(2));
    /// set.toggle(2);
    /// assert!(!set.contains(2));
    #[inline]
    pub const fn toggle(&mut self, i: u8) {
        let (high, low) = Self::indices(i);
        self.bytes[high] ^= low;
    }

    /// Returns the smallest value from the set, or `None` if the set is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let mut set = ByteSet::new();
    ///
    /// assert_eq!(set.min(), None);
    /// set |= 30..50;
    /// assert_eq!(set.min(), Some(30));
    #[allow(clippy::cast_possible_truncation)]
    #[inline]
    pub const fn min(&self) -> Option<u8> {
        match self.bytes {
            [0, 0, 0, 0] => None,
            [0, 0, 0, n] => Some(192 + n.trailing_zeros() as u8),
            [0, 0, n, _] => Some(128 + n.trailing_zeros() as u8),
            [0, n, _, _] => Some(64 + n.trailing_zeros() as u8),
            [n, _, _, _] => Some(n.trailing_zeros() as u8),
        }
    }

    /// Returns the largest value from the set, or `None` if the set is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytetable::ByteSet;
    ///
    /// let mut set = ByteSet::new();
    ///
    /// assert_eq!(set.max(), None);
    /// set |= 30..50;
    /// assert_eq!(set.max(), Some(49));
    #[allow(clippy::cast_possible_truncation)]
    #[inline]
    pub const fn max(&self) -> Option<u8> {
        match self.bytes {
            [0, 0, 0, 0] => None,
            [n, 0, 0, 0] => Some(63 - n.leading_zeros() as u8),
            [_, n, 0, 0] => Some(127 - n.leading_zeros() as u8),
            [_, _, n, 0] => Some(191 - n.leading_zeros() as u8),
            [_, _, _, n] => Some(255 - n.leading_zeros() as u8),
        }
    }

    /// Constructs a `ByteSet` containing all values contained in the range
    /// defined by the specified bounds.
    /// This is a low-level operation intended for use in constant contexts.
    /// Otherwise, `ByteSet::from` can be used on any range directly.
    ///
    /// # Examples
    ///
    /// ```
    /// use core::ops::Bound;
    /// use bytetable::ByteSet;
    ///
    /// const SET: ByteSet = ByteSet::from_bounds(Bound::Unbounded, Bound::Excluded(10));
    /// assert_eq!(SET, ByteSet::from(..10));
    #[inline]
    pub const fn from_bounds(start: Bound<u8>, end: Bound<u8>) -> Self {
        let [start0, start1, start2, start3] = match start {
            Bound::Unbounded => [0; 4],
            Bound::Included(n) => Self::mask_less_than(n),
            Bound::Excluded(n) => match n.checked_add(1) {
                Some(n) => Self::mask_less_than(n),
                None => [u64::MAX; 4],
            },
        };
        let [end0, end1, end2, end3] = match end {
            Bound::Unbounded => [u64::MAX; 4],
            Bound::Excluded(n) => Self::mask_less_than(n),
            Bound::Included(n) => match n.checked_add(1) {
                Some(n) => Self::mask_less_than(n),
                None => [u64::MAX; 4],
            },
        };
        Self {
            bytes: [
                !start0 & end0,
                !start1 & end1,
                !start2 & end2,
                !start3 & end3,
            ],
        }
    }

    /// The first value is the array index (0..4) of the associated `u64`.
    /// The second value is the bit position within that `u64`.
    #[inline]
    const fn indices(i: u8) -> (usize, u64) {
        (i as usize >> 6, 1 << (i & 63))
    }

    /// [`Self::from_bounds`] but with `Bound<&u8>` instead of `Bound<u8>`.
    /// This will not be necessary once [`std::ops::IntoBounds`] is stabilized.
    #[inline]
    const fn from_ref_bounds(start: Bound<&u8>, end: Bound<&u8>) -> Self {
        #[inline]
        const fn copy_bound(bound: Bound<&u8>) -> Bound<u8> {
            match bound {
                Bound::Included(n) => Bound::Included(*n),
                Bound::Excluded(n) => Bound::Excluded(*n),
                Bound::Unbounded => Bound::Unbounded,
            }
        }
        Self::from_bounds(copy_bound(start), copy_bound(end))
    }

    /// Constructs a set containing all values less than `i`.
    /// Equivalent to `ByteSet::from(..i)`.
    #[inline]
    const fn mask_less_than(i: u8) -> [u64; 4] {
        let (high, low) = Self::indices(i);
        let mask = low - 1;
        match high {
            0 => [mask, 0, 0, 0],
            1 => [u64::MAX, mask, 0, 0],
            2 => [u64::MAX, u64::MAX, mask, 0],
            3 => [u64::MAX, u64::MAX, u64::MAX, mask],
            // SAFETY: The maximum value of `u8` is 255. 255 >> 6 is 3.
            _ => unsafe { unreachable_unchecked() },
        }
    }

    /// Constructs an inclusive range from [`min`] to [`max`].
    /// If the set is empty, returns an exhausted range.
    ///
    /// [`min`]: Self::min
    /// [`max`]: Self::max
    #[inline]
    fn range(&self) -> RangeInclusive<u8> {
        if let (Some(min), Some(max)) = (self.min(), self.max()) {
            return min..=max;
        }
        let mut iter = 0..=0;
        iter.next();
        iter
    }

    /// Used for `ExactSizeIterator` implementations.
    #[inline]
    fn len_in_range(&self, range: &RangeInclusive<u8>) -> usize {
        if range.is_empty() || self.is_empty() {
            return 0;
        }
        (*self & range.clone()).len()
    }
}

impl fmt::Debug for ByteSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self).finish()
    }
}

impl Index<u8> for ByteSet {
    type Output = bool;

    #[inline]
    fn index(&self, index: u8) -> &Self::Output {
        if self.contains(index) {
            &true
        } else {
            &false
        }
    }
}

impl Not for ByteSet {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        let [a, b, c, d] = self.bytes;
        Self {
            bytes: [!a, !b, !c, !d],
        }
    }
}

macro_rules! impl_bitassign {
    ($i:ident, $f:ident $(, $single:ident, $l:literal)?) => {
        impl<T> $i<T> for ByteSet
        where
            ByteSet: From<T>,
        {
            #[inline]
            fn $f(&mut self, rhs: T) {
                let [l1, l2, l3, l4] = &mut self.bytes;
                let [r1, r2, r3, r4] = Self::from(rhs).bytes;
                l1.$f(r1);
                l2.$f(r2);
                l3.$f(r3);
                l4.$f(r4);
            }
        }

        $(
            impl $i<u8> for ByteSet {
                #[doc = "Alias for "]
                #[doc = $l]
                #[doc = "."]
                #[inline]
                fn $f(&mut self, rhs: u8) {
                    self.$single(rhs);
                }
            }
        )?
    };
}

impl_bitassign!(BitAndAssign, bitand_assign);
impl_bitassign!(BitOrAssign, bitor_assign, insert, "[`ByteSet::insert`]");
impl_bitassign!(BitXorAssign, bitxor_assign, toggle, "[`ByteSet::toggle`]");

macro_rules! impl_bit {
    ($i:ident, $f:ident $(, $single:ident, $l:literal)?) => {
        impl<T> $i<T> for ByteSet
        where
            ByteSet: From<T>,
        {
            type Output = Self;

            #[inline]
            fn $f(self, rhs: T) -> Self {
                let [l1, l2, l3, l4] = self.bytes;
                let [r1, r2, r3, r4] = Self::from(rhs).bytes;
                Self {
                    bytes: [l1.$f(r1), l2.$f(r2), l3.$f(r3), l4.$f(r4)],
                }
            }
        }

        $(
            impl $i<u8> for ByteSet {
                type Output = Self;

                #[doc = "Creates a copy of the set and performs "]
                #[doc = $l]
                #[doc = " on it with the specified argument.\n\nNote that this is potentially"]
                #[doc = " inefficient when chained, because it copies the entire 256-bit table for"]
                #[doc = " every operation. It is generally preferable to use "]
                #[doc = $l]
                #[doc = " directly instead."]
                #[inline]
                fn $f(self, rhs: u8) -> Self {
                    let mut copy = self;
                    copy.$single(rhs);
                    copy
                }
            }
        )?
    };
}

impl_bit!(BitAnd, bitand);
impl_bit!(BitOr, bitor, insert, "[`ByteSet::insert`]");
impl_bit!(BitXor, bitxor, toggle, "[`ByteSet::toggle`]");

impl Extend<u8> for ByteSet {
    #[inline]
    fn extend<T: IntoIterator<Item = u8>>(&mut self, iter: T) {
        for item in iter {
            self.insert(item);
        }
    }
}

impl<'a> Extend<&'a u8> for ByteSet {
    #[inline]
    fn extend<T: IntoIterator<Item = &'a u8>>(&mut self, iter: T) {
        for item in iter {
            self.insert(*item);
        }
    }
}

impl From<&[u8]> for ByteSet {
    #[inline]
    fn from(value: &[u8]) -> Self {
        Self::from_bytes(value)
    }
}

impl<const N: usize> From<&[u8; N]> for ByteSet {
    #[inline]
    fn from(value: &[u8; N]) -> Self {
        Self::from_bytes(value)
    }
}

impl<const N: usize> From<[u8; N]> for ByteSet {
    #[inline]
    fn from(value: [u8; N]) -> Self {
        Self::from_bytes(&value)
    }
}

impl From<(Bound<u8>, Bound<u8>)> for ByteSet {
    #[inline]
    fn from((start, end): (Bound<u8>, Bound<u8>)) -> Self {
        Self::from_bounds(start, end)
    }
}

impl From<RangeFull> for ByteSet {
    #[inline]
    fn from(_: RangeFull) -> Self {
        Self::full()
    }
}

macro_rules! impl_from_range {
    ($t:ty) => {
        impl From<$t> for ByteSet {
            #[inline]
            fn from(value: $t) -> Self {
                Self::from_ref_bounds(value.start_bound(), value.end_bound())
            }
        }
    };
}

impl_from_range!((Bound<&u8>, Bound<&u8>));

macro_rules! impl_from_ranges {
    ($t:ident) => {
        impl_from_range!(core::ops::$t<u8>);
        impl_from_range!(core::ops::$t<&u8>);
    };
}

impl_from_ranges!(Range);
impl_from_ranges!(RangeFrom);
impl_from_ranges!(RangeInclusive);
impl_from_ranges!(RangeTo);
impl_from_ranges!(RangeToInclusive);

impl FromIterator<u8> for ByteSet {
    #[inline]
    fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
        let mut set = Self::new();
        set.extend(iter);
        set
    }
}

impl<'a> FromIterator<&'a u8> for ByteSet {
    #[inline]
    fn from_iter<T: IntoIterator<Item = &'a u8>>(iter: T) -> Self {
        let mut set = Self::new();
        set.extend(iter);
        set
    }
}

macro_rules! impl_into_iter {
    ($t:ty) => {
        type Item = u8;
        type IntoIter = $t;

        #[inline]
        fn into_iter(self) -> Self::IntoIter {
            Self::IntoIter {
                inner: self.range(),
                set: self,
            }
        }
    };
}

macro_rules! impl_iter_inner {
    () => {
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            let exact = self.len();
            (exact, Some(exact))
        }

        #[inline]
        fn min(mut self) -> Option<u8> {
            self.next()
        }

        #[inline]
        fn max(mut self) -> Option<u8> {
            self.next_back()
        }

        #[inline]
        fn is_sorted(self) -> bool {
            true
        }
    };
}

macro_rules! impl_iter {
    ($t:ty, $f:expr) => {
        impl Iterator for $t {
            type Item = u8;

            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                self.inner.find(|&byte| self.set.contains(byte))
            }

            impl_iter_inner!();
        }

        impl DoubleEndedIterator for $t {
            #[inline]
            fn next_back(&mut self) -> Option<Self::Item> {
                self.inner.rfind(|&byte| self.set.contains(byte))
            }
        }

        impl ExactSizeIterator for $t {
            #[inline]
            fn len(&self) -> usize {
                self.set.len_in_range(&self.inner)
            }
        }

        impl FusedIterator for $t {}
    };
}

/// Iterates through the contents of a borrowed `ByteSet` in ascending order.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Iter<'a> {
    set: &'a ByteSet,
    inner: RangeInclusive<u8>,
}
impl_iter!(Iter<'_>, self.set.contains(byte));

impl<'a> IntoIterator for &'a ByteSet {
    impl_into_iter!(Iter<'a>);
}

/// Iterates through the contents of a `ByteSet` in ascending order.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct IntoIter {
    set: ByteSet,
    inner: RangeInclusive<u8>,
}
impl_iter!(IntoIter, self.set.contains(byte));

impl IntoIterator for ByteSet {
    impl_into_iter!(IntoIter);
}

/// This struct is created by [`ByteSet::extract_if`].
/// See its documentation for more.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct ExtractIf<'a, F> {
    set: &'a mut ByteSet,
    inner: RangeInclusive<u8>,
    pred: F,
}

impl<F> Iterator for ExtractIf<'_, F>
where
    F: FnMut(u8) -> bool,
{
    type Item = u8;

    #[inline]
    fn next(&mut self) -> Option<u8> {
        let next = self
            .inner
            .find(|&byte| self.set.contains(byte) && (self.pred)(byte))?;
        self.set.remove(next);
        Some(next)
    }

    impl_iter_inner!();
}

impl<F> ExactSizeIterator for ExtractIf<'_, F>
where
    F: FnMut(u8) -> bool,
{
    #[inline]
    fn len(&self) -> usize {
        self.set.len_in_range(&self.inner)
    }
}

impl<F> DoubleEndedIterator for ExtractIf<'_, F>
where
    F: FnMut(u8) -> bool,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner
            .rfind(|&byte| self.set.contains(byte) && (self.pred)(byte))
    }
}

impl<F> FusedIterator for ExtractIf<'_, F> where F: FnMut(u8) -> bool {}
