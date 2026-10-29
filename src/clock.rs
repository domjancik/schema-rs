//! Host-specific monotonic time. Evaluation time itself is explicitly supplied as dt.
#[cfg(not(all(target_arch = "wasm32", feature = "browser")))]
pub use std::time::Instant;
#[cfg(all(target_arch = "wasm32", feature = "browser"))]
pub use web_time::Instant;
