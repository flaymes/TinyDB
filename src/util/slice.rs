use crate::util::byte::compare;

use std::cmp::Ordering;
use std::ops::Index;
use std::ptr;
use std::slice;

/// Slice is a simple structure containing a pointer into some external
/// storage and a size.  The user of a Slice must ensure that the slice
/// is not used after the corresponding external storage has been
/// deallocated.
///
/// Multiple threads can invoke const methods on a Slice without
/// external synchronization, but if any of the threads may call a
/// non-const method, all threads accessing the same Slice must use
/// external synchronization.
#[derive(Clone, Debug)]
pub struct Slice {
    data: *const u8,
    size: usize,
}

impl Slice {
    pub fn new(data: *const u8, size: usize) -> Self {
        Self { data, size }
    }

    pub fn new_empty() -> Slice {
        Self::new(ptr::null(), 0)
    }

    #[inline]
    pub fn to_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.data, self.size) }
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }

    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.data
    }

    pub fn compare(&self, other: &Slice) -> Ordering {
        compare(self.to_slice(),other.to_slice())
    }

    #[inline]
    pub fn clear(&mut self) {
        self.data = ptr::null();
        self.size = 0;
    }
}

impl PartialEq for Slice {
    fn eq(&self, other: &Slice) -> bool {
        self.compare(other) == Ordering::Equal
    }
}

impl Index<usize> for Slice {
    type Output = u8;

    /// Return the ith byte in the referenced data
    fn index(&self, index: usize) -> &u8 {
        if index > self.size {
            panic!(
                "[slice] out of range. Slice size is [{}] but try to get [{}]",
                self.size, index
            );
        }
        unsafe { &*self.data.offset(index as isize) }
    }
}

impl<'a> From<&'a [u8]> for Slice {
    #[inline]
    fn from(v: &'a [u8]) -> Self {
        Slice::new(v.as_ptr(), v.len())
    }
}

impl From<Vec<u8>> for Slice {
    fn from(v: Vec<u8>) -> Self {
        Slice::new(v.as_ptr(), v.len())
    }
}

impl<'a> From<&'a Vec<u8>> for Slice {
    fn from(v: &'a Vec<u8>) -> Self {
        Slice::new(v.as_ptr(), v.len())
    }
}
