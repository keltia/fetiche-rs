//! Compiler for the Fetiche job language
//!
//! ```text
//! ## Description of the job & task language
//!
//! >NOTE: Highly subject to changes
//!
//! ```text
//! job "Fetch Opensky data" is
//!     schedule every(5mn) | at(DATE)[,at(DATE)]*  // ?
//!     
//!     message "Beginning"
//!
//!     res = fetch("opensky")
//!     
//!     message "transform"
//!     
//!     res = transform(Cat21)
//!     
//!     output("aeroscope.csv")
//! end
//! ```
//!
