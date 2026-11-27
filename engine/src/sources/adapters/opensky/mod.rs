//! Direct TCP and HTTP polling adapters for OpenSky sites.

mod device;
mod server;

use std::time::Duration;

pub use device::*;
pub use server::*;

/// The earliest requested timestamp is one hour before the current time.
const MAX_INTERVAL: i64 = 3600;

/// Expiration after the last access.
pub const CACHE_IDLE: Duration = Duration::from_secs(20);
/// Expiration after insertion.
pub const CACHE_MAX: Duration = Duration::from_secs(60);
/// Maximum number of recently forwarded snapshots.
pub const CACHE_SIZE: u64 = 20;
