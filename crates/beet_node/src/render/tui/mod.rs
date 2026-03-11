#[cfg(not(target_arch = "wasm32"))]
mod tui_plugin;
#[cfg(not(target_arch = "wasm32"))]
pub use tui_plugin::*;
#[cfg(not(target_arch = "wasm32"))]
mod input;
#[cfg(not(target_arch = "wasm32"))]
pub use input::*;
mod renderer;
pub use renderer::*;
mod span_map;
pub use span_map::*;
