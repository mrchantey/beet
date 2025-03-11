//! Module containing all plugins to be applied to an [`RsxRoot`]
#[cfg(feature = "fs")]
mod fs_src_plugin;
mod slots_plugin;
#[cfg(feature = "fs")]
pub use fs_src_plugin::*;
pub use slots_plugin::*;
#[cfg(feature = "css")]
mod scoped_style_plugin;
#[cfg(feature = "css")]
pub use scoped_style_plugin::*;

use crate::prelude::*;
use anyhow::Result;


/// Trait for plugins that will mutate an [`RsxRoot`]
pub trait RsxPlugin {
	/// Consume self and apply the mod to the root
	fn apply(self, root: &mut RsxRoot) -> Result<()>;
}

impl RsxRoot {
	/// Apply default rsx plugins:
	/// - [FsSrcPlugin]
	/// - [ScopedStylePlugin]
	/// - [SlotsPlugin]
	pub fn apply_default_plugins(&mut self) -> Result<()> {
		#[cfg(feature = "fs")]
		FsSrcPlugin::default().apply(self)?;
		#[cfg(feature = "css")]
		ScopedStylePlugin::default().apply(self)?;
		SlotsPlugin::default().apply(self)?;
		Ok(())
	}
}
