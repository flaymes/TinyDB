use libc::{c_int, c_void, size_t};
use std::cmp::{Ordering, min, max};

extern "C" {
    fn memcmp(cx: *const c_void, ct: *const c_void, n: size_t) -> c_int;
}

#[inline]
pub fn compare(b1: &[u8], b2: &[u8]) -> Ordering {
    if b1.is_empty() && b2.is_empty() {
        return Ordering::Equal;
    }
    if b1.is_empty() {
        return Ordering::Less;
    }
    if b2.is_empty() {
        return Ordering::Greater;
    }

    let n = max(b1.len(), b2.len());

    unsafe {
        let result = Some(memcmp(
            b1.as_ptr() as *const c_void,
            b2.as_ptr() as *const c_void,
            n as size_t,
        ));
        match result {
            Some(x) if x > 0 => Ordering::Greater,
            Some(x) if x == 0 => Ordering::Equal,
            Some(x) if x < 0 => Ordering::Less,
            Some(_) | None => panic!("invalid memcmp returning"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn test_compare() {
        let mut tests = vec![
            (vec![], vec![], Ordering::Equal),
            (vec![], vec![1u8], Ordering::Less),
            (vec![2u8], vec![], Ordering::Greater),
            (vec![1u8, 2u8, 3u8], vec![1u8, 2u8, 3u8], Ordering::Equal),
            (vec![1u8, 2u8, 3u8], vec![1u8, 3u8, 2u8], Ordering::Less),
            (vec![1u8, 3u8, 3u8], vec![1u8, 2u8, 2u8], Ordering::Greater),
        ];

        for (b1, b2, expect) in tests.drain(..) {
            assert_eq!(compare(b1.as_slice(), b2.as_slice()), expect);
        }
    }
}