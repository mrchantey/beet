mod socket;
pub use socket::Message;
pub use socket::*;
mod socket_server;
pub use socket_server::*;
#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
pub(self) mod impl_tungstenite;
#[cfg(target_arch = "wasm32")]
pub(self) mod impl_web_sys;
