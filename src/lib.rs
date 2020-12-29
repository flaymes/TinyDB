extern crate log;
extern crate byteorder;
extern crate libc;
extern crate rand;

#[macro_use]
pub mod util;
pub mod config;
pub mod db;
pub mod mem;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
