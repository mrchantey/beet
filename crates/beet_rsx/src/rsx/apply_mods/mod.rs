//! Module containing all mods to be applied to an [`RsxRoot`]
mod slots_visitor;
pub use slots_visitor::*;
#[cfg(feature = "css")]
mod scoped_style;
#[cfg(feature = "css")]
pub use scoped_style::*;

use crate::prelude::*;
use anyhow::Result;


impl RsxRoot {
	/// Apply all rsx mutations
	/// - [ScopedStyle::apply]
	/// - [SlotsVisitor::apply]
	pub fn apply_default_mods(&mut self) -> Result<()> {
		#[cfg(feature = "css")]
		ScopedStyle::default().apply(self)?;
		SlotsVisitor::apply(self)?;
		Ok(())
	}
}
