mod socket;
pub use socket::Message;
pub use socket::*;
mod socket_server;
pub use socket_server::*;
mod socket_server_plugin;
pub use socket_server_plugin::*;
#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
mod impl_tungstenite;
#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
pub use impl_tungstenite::SocketServerStatus;
#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
pub(crate) use impl_tungstenite::start_tungstenite_server;
#[cfg(target_arch = "wasm32")]
pub(self) mod impl_web_sys;
