#[cfg(not(target_arch = "wasm32"))]
mod stdio;
#[cfg(not(target_arch = "wasm32"))]
pub use stdio::*;
#[cfg(feature = "markdown")]
mod markdown;
#[cfg(feature = "markdown")]
pub use markdown::*;
mod html;
pub use html::*;
mod mime;
pub use mime::*;
#[cfg(all(feature = "http", not(target_arch = "wasm32")))]
mod http;
#[cfg(all(feature = "http", not(target_arch = "wasm32")))]
pub use http::*;
#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
mod tui;
#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
pub use tui::*;
