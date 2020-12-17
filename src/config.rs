pub struct Config {
    /// Directory to store the main data in. Should exist and be writable.
    pub dir: String,
    /// Directory to store the value log in. Can be the same as Dir. Should exist and be writable.
    pub value_dir: String,
}