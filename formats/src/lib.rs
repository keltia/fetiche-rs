//! Definition of a data formats
//!
//! This module makes the link between the shared output formats `Cat21` and the different
//! input formats defined in the other modules.
//!
//! To add a new formats, insert here the different hooks (`Source`, etc.) & names and a `FORMAT.rs`
//! file which will define the input formats and the transformations needed.
//!

// Re-export for convenience
//
pub use aeroscope::*;
pub use asd::*;
pub use asterix::*;
pub use avionix::*;
pub use common::*;
pub use dronepoint::*;
#[cfg(feature = "flightaware")]
pub use flightaware::*;
pub use format::*;
pub use opensky::*;
#[cfg(feature = "safesky")]
pub use safesky::*;

mod aeroscope;
mod asd;
mod asterix;
mod avionix;
mod common;
mod dronepoint;
#[cfg(feature = "flightaware")]
mod flightaware;
mod format;
mod opensky;
#[cfg(feature = "safesky")]
mod safesky;
pub mod senhive;

/// Generate a converter called `$name` which takes `&str` and
/// output a `Vec<$to>`.  `input` is deserialized from JSON as
/// `$from`.
///
/// Uses `$to::from()` for each format.
///
/// You will need to `use` these in every file you use the macro
/// ```no_run
/// use eyre::Result;
/// use log::debug;
/// ```
/// or
/// ```no_run
/// use eyre::Result;
/// use tracing::debug;
/// ```
///
/// Takes 3 arguments:
///
/// - name of the `fn` to create
/// - name of the input `struct`
/// - name of the output type like `Cat21`
///
#[macro_export]
macro_rules! convert_to {
    ($name:ident, $from:ident, $to:ident) => {
        impl $to {
            #[doc = concat!("This is ", stringify!($name), " which convert a json string into a ", stringify!($to), "object")]
            ///
            #[tracing::instrument]
            pub fn $name(input: &str) -> Result<Vec<$to>> {
                debug!("IN={:?}", input);
                let stream = ::std::io::BufReader::new(input.as_bytes());
                let res = ::serde_json::Deserializer::from_reader(stream).into_iter::<$from>();

                let res: Vec<_> = res
                    .filter(|l| l.is_ok())
                    .enumerate()
                    .inspect(|(n, f)| debug!("cnt={}/{:?}", n, f.as_ref().unwrap()))
                    .map(|(_cnt, rec)| {
                        $to::from(&rec.unwrap())
                    })
                    .collect();
                debug!("res={:?}", res);
                Ok(res)
            }
        }
    };
}

