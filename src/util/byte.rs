use libc::{c_int, c_void, size_t};
use std::cmp::{Ordering, min};

extern "C" {
    fn memcmp(cx: *const c_void, ct: *const c_void, n: size_t) -> c_int;
}

#[inline]
pub fn compare(b1: &[u8], b2: &[u8]) -> Ordering {
    if b1.is_empty() && b2.is_empty() {
        return Ordering::Equal;
    }
    let n = min(b1.len(), b2.len());

    unsafe {
        let result = memcmp(
            b1.as_ptr() as *const c_void,
            b2.as_ptr() as *const c_void,
            n as size_t,
        );
        match result {
            -1 => Ordering::Less,
            0 => Ordering::Equal
            1 => Ordering::Greater,
            _ => panic!("invalid memcmp return [{}]", result),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn test_compare() {
        let s1 = vec![1u8, 2u8, 3u8];
        let s2 = vec![1u8, 3u8, 2u8];
        assert_eq!(Ordering::Less,compare(s1,s2));
    }
}