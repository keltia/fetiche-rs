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
//!     fetch("opensky")
//!     
//!     message "Transform into Cat21"
//!     
//!     into(Cat21)
//!     
//!     output("aeroscope.csv")
//! end
//! ```
//!

//use nom::{complete::tag, IResult};

// fn parse_message(input: &str) -> IResult<&str, &str> {}
//
// struct Schedule {}
//
// fn parse_schedule(input: &str) -> IResult<&str, Schedule> {}
//
// fn parse_fetch(input: &str) -> IResult<&str, &str> {}
