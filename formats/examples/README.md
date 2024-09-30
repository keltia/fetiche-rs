//! Alternative version using `arrow2` instead of arrow/parquet:etc.
//!
//! Benchmarks (hyperfine): 55990279 bytes, 136373 records -- Mac Studio M2 Pro, dev builds
//!
//! Uncompressed size:
//! Zstd, default compression 3
//! ```text
//! Benchmark 1: cargo run --example parquet25 2023-jul-nov2
//!   Time (mean ± σ):     10.393 s ±  0.031 s    [User: 9.965 s, System: 0.168 s]
//!   Range (min … max):   10.361 s … 10.456 s    10 runs
//! ```
//! Size: 2881674
//!
//! Zstd, compression 8
//! ```text
//! Benchmark 1: cargo run --example parquet25 2023-jul-nov2
//!   Time (mean ± σ):     11.163 s ±  0.033 s    [User: 10.726 s, System: 0.175 s]
//!   Range (min … max):   11.131 s … 11.225 s    10 runs
//! ```
//! Size: 2558422
//!
//! Brotli, default level
//! ```text
//! Benchmark 1: cargo run --example parquet25 2023-jul-nov2
//!   Time (mean ± σ):     11.197 s ±  0.196 s    [User: 10.755 s, System: 0.167 s]
//!   Range (min … max):   11.010 s … 11.517 s    10 runs
//! ```
//! Size: 2943996
//!
//! For fun:
//! Zstd, max compression level 22
//! ```text
//! Benchmark 1: cargo run --example parquet25 2023-jul-nov2
//!   Time (mean ± σ):     35.282 s ±  0.304 s    [User: 33.202 s, System: 1.802 s]
//!   Range (min … max):   35.044 s … 36.099 s    10 runs
//! ```
//! Size: 2048914
