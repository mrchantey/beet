#![feature(let_chains)]
pub mod events;
pub mod extensions;
pub mod networking;
pub mod replication;
#[cfg(feature = "tokio")]
pub mod tokio_client;
#[cfg(target_arch = "wasm32")]
pub mod web_transport;

pub mod prelude {
	pub use crate::events::*;
	pub use crate::extensions::*;
	pub use crate::networking::*;
	pub use crate::replication::*;
	#[cfg(feature = "tokio")]
	pub use crate::tokio_client::*;
	#[cfg(target_arch = "wasm32")]
	pub use crate::web_transport::*;
}
