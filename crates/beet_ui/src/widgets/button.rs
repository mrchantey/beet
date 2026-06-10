//! Button widgets: `Button`, `IconButton`, `Link`.
//!
//! Each emits the semantic [`classes::BTN`] base plus a variant class; the
//! active rule set (Material Design 3 today) maps those to design tokens. A
//! [`Link`] is an `<a>` styled as a button — the rules match any element, so a
//! hyperlink picks up the same look as a `<button>`.
use crate::prelude::*;
use beet_core::prelude::*;

/// Visual emphasis variant, mapped one-to-one onto a semantic class.
#[derive(Debug, Default, Clone, PartialEq, Reflect)]
pub enum ButtonVariant {
	/// High-emphasis primary action (`btn-filled`).
	#[default]
	Filled,
	/// Medium emphasis with a visible border (`btn-outlined`).
	Outlined,
	/// Lowest emphasis, no container (`btn-text`).
	Text,
	/// Medium emphasis using the secondary container (`btn-tonal`).
	Tonal,
	/// Medium emphasis with a shadow (`btn-elevated`).
	Elevated,
	/// Filled using the secondary color (`btn-secondary`).
	Secondary,
	/// Filled using the tertiary color (`btn-tertiary`).
	Tertiary,
	/// Destructive action using the error color (`btn-error`).
	Error,
}

impl ButtonVariant {
	/// The semantic class name for this variant.
	pub fn class(&self) -> ClassName {
		match self {
			ButtonVariant::Filled => classes::BTN_FILLED,
			ButtonVariant::Outlined => classes::BTN_OUTLINED,
			ButtonVariant::Text => classes::BTN_TEXT,
			ButtonVariant::Tonal => classes::BTN_TONAL,
			ButtonVariant::Elevated => classes::BTN_ELEVATED,
			ButtonVariant::Secondary => classes::BTN_SECONDARY,
			ButtonVariant::Tertiary => classes::BTN_TERTIARY,
			ButtonVariant::Error => classes::BTN_ERROR,
		}
	}
}

/// A styled `<button>`; its content is the default slot's children.
#[template]
pub fn Button(variant: ButtonVariant) -> impl Bundle {
	rsx! {
		<button {Classes::new([classes::BTN, variant.class()])}><Slot/></button>
	}
}

/// A `<button>` sized for a single glyph (`btn-icon`); the slot is the icon.
#[template]
pub fn IconButton(variant: ButtonVariant) -> impl Bundle {
	rsx! {
		<button {Classes::new([classes::BTN, classes::BTN_ICON, variant.class()])}>
			<Slot/>
		</button>
	}
}

/// An `<a>` hyperlink styled as a button; its content is the default slot.
#[template]
pub fn Link(#[prop(into)] href: String, variant: ButtonVariant) -> impl Bundle {
	rsx! {
		<a {Classes::new([classes::BTN, variant.class()])} href={href}>
			<Slot/>
		</a>
	}
}
