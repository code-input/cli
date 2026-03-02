#[cfg(feature = "types")]
mod core {
    pub mod types;
}

#[cfg(feature = "types")]
pub use core::types::*;

#[cfg(not(feature = "types"))]
pub mod core;
#[cfg(not(feature = "types"))]
pub mod utils;

// LSP server module (requires full features - not just types)
#[cfg(all(not(feature = "types"), feature = "tokio", feature = "tower-lsp"))]
pub mod lsp;
