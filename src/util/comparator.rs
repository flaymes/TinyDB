use super::slice::Slice;

use std::cmp::Ordering;

/// A Comparator object provides a total order across T that are
/// used as keys in an sstable or a database.  A Comparator implementation
/// must be thread-safe since we may invoke its methods concurrently
/// from multiple threads.
pub trait Comparator<T> {
    /// Three-way comparison. Returns value:
    ///   `Ordering::Less`    iff `self` < `b`
    ///   `Ordering::Equal`   iff `self` = `b`
    ///   `Ordering::Greater` iff `self` > `b`
    fn compare(&self, a: &T, b: &T) -> Ordering;

    /// The name of the comparator.  Used to check for comparator
    /// mismatches (i.e., a DB created with one comparator is
    /// accessed using a different comparator.
    ///
    /// The client of this package should switch to a new name whenever
    /// the comparator implementation changes in a way that will cause
    /// the relative ordering of any two keys to change.
    ///
    /// Names starting with "wickdb." are reserved and should not be used
    /// by any clients of this package.
    fn name(&self) -> &str;
}

pub struct BytewiseComparator {}

impl BytewiseComparator {
    pub fn new() -> BytewiseComparator {
        BytewiseComparator {}
    }
}

impl Comparator<Slice> for BytewiseComparator{
    #[inline]
    fn compare(&self, a: &Slice, b: &Slice) -> Ordering {
        a.compare(b)
    }

    #[inline]
    fn name(&self) -> &str {
        "tinydb.BytewiseComparator"
    }
}