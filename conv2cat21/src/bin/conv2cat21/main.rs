//! Main entry point for Conv2cat21

#![deny(warnings, missing_docs, trivial_casts, unused_qualifications)]
#![forbid(unsafe_code)]

use conv2cat21::application::APP;

/// Boot Conv2cat21
fn main() {
    abscissa_core::boot(&APP);
}
