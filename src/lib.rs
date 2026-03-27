//! Mutable manipulation of [`ByteString`]s for creating UTF-8 strings that can be cheaply cloned
//! and shared across threads without allocating memory.
//!
//! Internally, a `ByteStringMut` is a wrapper around a [`BytesMut`] buffer from the [`bytes`]
//! crate. It offers most of the same functionality, except it prevents the underlying data from
//! becoming invalid UTF-8. Due to this, it can safely be used to construct strings, as well as
//! dereferenced as a `&str` or `&mut str`.

#![no_std]

use core::mem::MaybeUninit;

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteTable<T> {
    table: [T; 256],
}

impl<T: Clone + Default> Default for ByteTable<T> {
    fn default() -> Self {
        let default = T::default();
        Self::generate(|_| default.clone())
    }
}

impl<T> ByteTable<T> {
    #[inline]
    pub const fn new(table: [T; 256]) -> Self {
        Self { table }
    }

    #[inline]
    pub fn into_array(self) -> [T; 256] {
        self.table
    }

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
        unsafe { Self::assume_init(table) }
    }

    #[inline]
    pub fn get(&self, i: u8) -> &T {
        unsafe { self.table.get_unchecked(i as usize) }
    }

    #[inline]
    pub fn get_mut(&mut self, i: u8) -> &mut T {
        unsafe { self.table.get_unchecked_mut(i as usize) }
    }

    #[allow(clippy::needless_pass_by_value)]
    const unsafe fn assume_init(table: [MaybeUninit<T>; 256]) -> Self {
        Self {
            table: unsafe { table.as_ptr().cast::<[T; 256]>().read() },
        }
    }
}
