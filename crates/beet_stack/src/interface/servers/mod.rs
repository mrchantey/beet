//! Servers originate a [`Request`] and handle the corresponding [`Response`]
mod cli_server;
pub use cli_server::*;
mod repl_server;
pub use repl_server::*;
