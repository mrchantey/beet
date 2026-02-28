#[cfg(not(target_arch = "wasm32"))]
mod stdio;
#[cfg(not(target_arch = "wasm32"))]
pub use stdio::*;
mod markdown;
pub use markdown::*;
mod html;
pub use html::*;
mod mime;
pub use mime::*;
#[cfg(feature = "http")]
mod http;
#[cfg(feature = "http")]
pub use http::*;
#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
mod tui;
#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
pub use tui::*;
