pub mod arrow2;
pub mod datafusion;

#[derive(Debug)]
pub struct Options {
    pub delim: u8,
    pub header: bool,
}
