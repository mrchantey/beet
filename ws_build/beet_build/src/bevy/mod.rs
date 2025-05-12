#[cfg(not(target_arch = "wasm32"))]
mod bevy_template_reloader;
#[cfg(not(target_arch = "wasm32"))]
pub use bevy_template_reloader::*;
