#![feature(let_chains)]
pub mod extensions;
#[cfg(feature = "tokio-client")]
pub mod tokio_client;
pub mod networking;
pub mod replication;
#[cfg(target_arch = "wasm32")]
pub mod web_client;

pub mod prelude {
	pub use crate::extensions::*;
	#[cfg(feature = "tokio-client")]
	pub use crate::tokio_client::*;
	pub use crate::networking::*;
	pub use crate::replication::*;
	#[cfg(target_arch = "wasm32")]
	pub use crate::web_client::*;
}
