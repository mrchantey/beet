mod codegen;
mod components;
mod layouts;
mod types;

#[path = "codegen/client_islands.rs"]
#[cfg(feature = "client")]
pub mod client_islands;

pub use codegen::actions;

pub mod prelude {
	#[cfg(feature = "client")]
	pub use crate::client_islands::*;
	pub use crate::codegen::actions;
	pub use crate::codegen::actions::ActionsPlugin;
	pub use crate::codegen::docs::DocsPlugin;
	pub use crate::codegen::pages::PagesPlugin;
	pub use crate::codegen::*;
	pub use crate::components::*;
	pub use crate::layouts::*;
	pub use crate::types::*;
}
