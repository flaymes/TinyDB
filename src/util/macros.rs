macro_rules! invariant {
    ($condition:expr,$($arg:tt)*) => {
        if !$condition {
            panic!($($arg)*);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invariant_equal() {
        invariant!(true,"equal");
    }

    #[test]
    #[should_panic]
    fn test_invarint_should_panic(){
        invariant!(1 == 2, "equal");
    }

}