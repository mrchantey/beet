mod cli_server;
#[cfg(all(
	feature = "server",
	not(feature = "lambda"),
	not(target_arch = "wasm32")
))]
mod hyper_server;
#[cfg(all(feature = "lambda", not(target_arch = "wasm32")))]
mod lambda_server;

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
mod http_server;
mod server_plugin;
pub use cli_server::*;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub use http_server::*;
#[cfg(all(
	feature = "server",
	not(feature = "lambda"),
	not(target_arch = "wasm32")
))]
use hyper_server::*;
#[cfg(all(feature = "lambda", not(target_arch = "wasm32")))]
use lambda_server::*;
pub use server_plugin::*;
