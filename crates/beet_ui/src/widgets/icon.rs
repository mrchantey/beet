//! Line icons: [`Icon`] draws an [`IconName`] inline in the text colour on the
//! web, and as a glyph in the terminal.
//!
//! The drawings are [Lucide](https://lucide.dev)'s (ISC licence), one stroke
//! weight on one 24 unit grid, so every icon matches every other whatever font
//! the page wears, where a glyph (`☰`) takes its weight from the font. Each is
//! Lucide's own paths, a circle written as two arcs, so an icon is one list of
//! path data; adding one is a variant, its paths copied from the Lucide source,
//! and its glyph.
use crate::prelude::*;
use beet_core::prelude::*;

/// An icon [`Icon`] draws, from the Lucide set.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum IconName {
	/// Three bars, a menu or a drawer (Lucide `menu`).
	#[default]
	Menu,
	/// A half filled circle, the scheme following the system (Lucide
	/// `contrast`).
	Contrast,
	/// The light scheme (Lucide `sun`).
	Sun,
	/// The dark scheme (Lucide `moon`).
	Moon,
}

impl IconName {
	/// The icon's path data on Lucide's 24 unit grid.
	pub fn paths(self) -> &'static [&'static str] {
		match self {
			Self::Menu => &["M4 6h16", "M4 12h16", "M4 18h16"],
			Self::Contrast => &[
				"M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0",
				"M12 18a6 6 0 0 0 0-12v12z",
			],
			Self::Sun => &[
				"M8 12a4 4 0 1 0 8 0a4 4 0 1 0 -8 0",
				"M12 2v2",
				"M12 20v2",
				"m4.93 4.93 1.41 1.41",
				"m17.66 17.66 1.41 1.41",
				"M2 12h2",
				"M20 12h2",
				"m6.34 17.66-1.41 1.41",
				"m19.07 4.93-1.41 1.41",
			],
			Self::Moon => &["M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"],
		}
	}

	/// The terminal's stand-in, a glyph a monospace font carries.
	pub fn glyph(self) -> &'static str {
		match self {
			Self::Menu => "三",
			Self::Contrast => "◐",
			Self::Sun => "☀",
			Self::Moon => "☾",
		}
	}
}

/// `<Icon name=IconName::Menu/>`: the named icon, an inline svg of one size
/// wherever it sits, so a row of controls wears matching icons, stroked in the
/// text colour and hidden from assistive technology, since the control holding
/// it carries the label. The terminal shows the [glyph](IconName::glyph)
/// instead.
#[template]
pub fn Icon(name: IconName) -> impl Bundle {
	let paths = name
		.paths()
		.iter()
		.map(|path| rsx! { <path d={*path}/> })
		.collect::<Vec<_>>();
	rsx! {
		<span {Classes::new([classes::ICON])}>
			<svg
				{Classes::new([classes::TERMINAL_HIDDEN])}
				viewBox="0 0 24 24"
				width="1.25rem"
				height="1.25rem"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"
			>{paths}</svg>
			<span {Classes::new([classes::TERMINAL_ONLY])}>{name.glyph()}</span>
		</span>
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn draws_its_paths() {
		let mut world = world_ext::ui_world();
		let root = world
			.spawn_template(rsx! { <Icon name=IconName::Menu/> })
			.unwrap()
			.id();
		crate::widgets::test_ext::render_world(&mut world, root)
			.xpect_contains("viewBox=\"0 0 24 24\"")
			.xpect_contains("d=\"M4 12h16\"")
			.xpect_contains("三");
	}
}
