mod app_router_plugin;
mod app_runner;
mod route_instance;
#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
pub use axum_runner::*;
#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
mod axum_runner;
mod bundle_layer;
mod route_layer;
pub use app_router_plugin::*;
pub use app_runner::*;
pub use bundle_layer::*;
pub use route_instance::*;
pub use route_layer::*;
mod route_handler;
pub use route_handler::*;
mod clone_plugin;
pub use clone_plugin::*;
mod router;
// #[cfg(feature = "axum")]
// pub use axum_impl::*;
pub use router::*;
