#[macro_export]
macro_rules! invarint {
    ($condition:expr, $($arg:tt)*) => {
        if !$condition {
            panic!($($arg)*);
        }
    };
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_invarint_equal() {
        invarint!(true, "equal");
    }

    #[test]
    #[should_panic]
    fn test_invarint_should_panic() {
        invarint!(1 == 2, "equal");
    }
}