use crate::config::Config;
use crate::util::error::TinyError;
use crate::util::slice::Slice;

pub struct DB {}

pub fn open_db(config: Config) -> DB {
    DB::new()
}


impl DB {
    pub fn new() -> DB {
        DB {}
    }
    pub fn write(&self, key: Slice, value: Slice) -> Result<(), TinyError> {
        println!("[write] key: {:?}, value: {:?}", &key, &value);
        Ok(())
    }
    pub fn get(&self, key: Vec<&u8>) -> Option<Slice> {
        println!("[get] key: {:?}", &key);
        None
    }
}