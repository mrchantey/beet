#[cfg(feature = "server")]
mod default_router;
mod endpoint_tree;
mod param_pattern;
mod route_query;
pub use collect_html::*;
#[cfg(feature = "server")]
pub use default_router::*;
pub use endpoint_tree::*;
pub use param_pattern::*;
pub use path_pattern::*;
pub use route_query::*;
pub use router_runner::*;
pub use server_action_request::*;
mod collect_html;
mod path_pattern;
mod router_plugin;
mod router_runner;
mod server_action_request;
pub use router_plugin::*;
