// mod app_router_args;
// pub use app_router_args::*;
mod app_error;
pub use app_error::*;
mod app_state;
pub use app_state::*;
mod route_path;
pub use route_path::*;
mod route_info;
pub use route_info::*;
mod bundle_route;
pub use bundle_route::*;
#[cfg(feature = "nightly")]
/// implement `BundleRoute` for handlers with any number of parameters
mod bundle_route_nightly;
#[cfg(not(feature = "nightly"))]
/// implement `BundleRoute` for handlers with a single parameter,
/// requiring the use of tuples for multiple extractors
mod bundle_route_stable;
