//! Form-control classes and their Material Design 3 rules.
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use crate::style::material::*;

// ── Class names ─────────────────────────────────────────────────────────────────
pub const INPUT: ClassName = ClassName::new_static("input");
pub const INPUT_OUTLINED: ClassName = ClassName::new_static("input-outlined");
pub const INPUT_FILLED: ClassName = ClassName::new_static("input-filled");
pub const INPUT_TEXT: ClassName = ClassName::new_static("input-text");
pub const CHECKBOX: ClassName = ClassName::new_static("checkbox");
pub const SELECT: ClassName = ClassName::new_static("select");
pub const SELECT_OUTLINED: ClassName = ClassName::new_static("select-outlined");
pub const SELECT_FILLED: ClassName = ClassName::new_static("select-filled");
pub const SELECT_TEXT: ClassName = ClassName::new_static("select-text");
pub const SELECT_DROPDOWN: ClassName = ClassName::new_static("select-dropdown");
pub const SELECT_OPTION: ClassName = ClassName::new_static("select-option");
pub const ERROR_TEXT: ClassName = ClassName::new_static("error-text");
pub const ERROR: ClassName = ClassName::new_static("error");

// ── Rules ─────────────────────────────────────────────────────────────────────

/// `<form>` layout - a vertical stack whose fields stretch to the form width, so
/// a bare `<form>` of labels and inputs reads as key-over-value rows with a gap
/// between each field. Works on both targets.
pub fn form_layout() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("form"))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::FlexDirectionProp, Direction::Vertical)
		.with_value(common_props::AlignItemsProp, AlignItems::Stretch)
		.with_value(common_props::RowGapProp, Length::Rem(1.0))
}

/// Field `<label>` - the key sitting just above its control, medium weight so
/// the key reads as a heading for its value.
///
/// A column rather than a block, because a `<label>` here *wraps* its control
/// (the association HTML gives without a `for` attribute, and the shape
/// [`DynamicForm`](crate::prelude::DynamicForm) generates): as a block its key
/// text and its input are two inline items on one line, so the key runs
/// straight into the value. `start` rather than the flex default, so an
/// unsized control (a checkbox) stays its own width instead of stretching to
/// the field's.
pub fn label_field() -> Rule {
	Rule::new()
		.with_selector(Selector::tag("label"))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::FlexDirectionProp, Direction::Vertical)
		.with_value(common_props::AlignItemsProp, AlignItems::Start)
		.with_token(common_props::FontWeightProp,typography::WeightMedium).unwrap()
		.with_value(common_props::MarginProp, Spacing {
			bottom: Length::Rem(0.25),
			..Spacing::DEFAULT
		})
}

/// A `<button>` inside a form shrink-wraps instead of stretching to the field
/// width.
///
/// [`form_layout`]'s stretch is for *fields*, which read as full-width rows; a
/// button is a target the user aims at, and stretched it reads as a coloured
/// band across the form rather than as a control. A descendant selector, since
/// a generated form nests its structural buttons (a list's `add`, a row's
/// `remove`) inside the collection they edit.
pub fn form_button() -> Rule {
	Rule::new()
		.with_selector(Selector::descendant(
			Selector::tag("form"),
			Selector::tag("button"),
		))
		.with_value(common_props::AlignSelfProp, AlignSelf::Start)
}

/// Shared baseline for `.input` text fields and text areas. A fixed `15rem`
/// width gives a consistent, comfortable measure on both targets rather than
/// the browser's `size`-derived default (and the terminal's content-hugging
/// box); a `<form>`'s stretch still grows wider fields when needed.
pub fn input_base() -> Rule {
	Rule::new()
		.with_selector(Selector::class(INPUT))
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(TypographyProps,typography::BodyLarge).unwrap()
		.with_token(ShapeProps,geometry::ShapeExtraSmall).unwrap()
		.with_value(common_props::Width, Length::Rem(15.))
		.with_value(common_props::Padding, Spacing::all(Length::Rem(0.5)))
}

/// Terminal controls - trims the vertical padding so a text field or a select is
/// three rows (border, value, border) rather than five.
///
/// The same trade [`app_bar_terminal`](super::app_bar_terminal) makes: `0.5rem`
/// rounds to a whole cell row on each side, and a form of five-row fields
/// pushes everything below it off a terminal window for padding no one reads.
/// The horizontal inset is kept, since that is what separates the value from
/// its border.
pub fn control_terminal() -> Rule {
	Rule::new()
		.with_media(MediaQuery::Terminal)
		.with_selector(
			Selector::class(INPUT).merge_any(Selector::class(SELECT)),
		)
		.with_value(common_props::Padding, Spacing {
			left: Length::Rem(0.5),
			right: Length::Rem(0.5),
			..Spacing::DEFAULT
		})
}

/// Outlined input - visible border, transparent fill.
pub fn input_outlined() -> Rule {
	Rule::new()
		.with_selector(Selector::class(INPUT_OUTLINED))
		.with_token(common_props::BorderColorProp,colors::Outline).unwrap()
		.with_token(common_props::OutlineWidth,geometry::OutlineWidthThin).unwrap()
}

/// Filled input - shaded container, no border.
pub fn input_filled() -> Rule {
	Rule::new()
		.with_selector(Selector::class(INPUT_FILLED))
		.with_token(common_props::BackgroundColor,colors::SurfaceContainerHighest).unwrap()
}

/// Text input - lowest emphasis, underline only (no container).
pub fn input_text() -> Rule {
	Rule::new()
		.with_selector(Selector::class(INPUT_TEXT))
		.with_token(common_props::BorderColorProp,colors::OutlineVariant).unwrap()
}

/// Focused input - primary-colored border. Compound selector `.input:focus`.
pub fn input_focus() -> Rule {
	Rule::new()
		.with_selector(Selector::AllOf(vec![
			Selector::class(INPUT),
			Selector::state(ElementState::Focused),
		]))
		.with_token(common_props::BorderColorProp,colors::Primary).unwrap()
}

/// Checkbox control. The browser draws its own native box, and the terminal
/// paints a `[x]`/`[ ]` marker from the bound value, so the base rule only pins
/// the surface foreground the marker inherits.
pub fn checkbox_base() -> Rule {
	Rule::new()
		.with_selector(Selector::class(CHECKBOX))
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
}

/// Focused checkbox - primary-colored. The marker *is* the control on the
/// terminal, so focus tints the glyph rather than a border.
pub fn checkbox_focus() -> Rule {
	Rule::new()
		.with_selector(Selector::AllOf(vec![
			Selector::class(CHECKBOX),
			Selector::state(ElementState::Focused),
		]))
		.with_token(common_props::ForegroundColor,colors::Primary).unwrap()
}

/// Shared baseline for `.select` elements, matching the `.input` width so a
/// form's controls line up at a consistent measure. Positioned so the open
/// control's `.select-dropdown` anchors to it.
pub fn select_base() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SELECT))
		.with_token(common_props::ForegroundColor,colors::OnSurface).unwrap()
		.with_token(TypographyProps,typography::BodyLarge).unwrap()
		.with_token(ShapeProps,geometry::ShapeExtraSmall).unwrap()
		.with_value(common_props::Width, Length::Rem(15.))
		.with_value(common_props::Padding, Spacing::all(Length::Rem(0.5)))
		.with_value(common_props::PositionProp, Position::Relative)
}

/// Outlined select - visible border, transparent fill.
pub fn select_outlined() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SELECT_OUTLINED))
		.with_token(common_props::BorderColorProp,colors::Outline).unwrap()
		.with_token(common_props::OutlineWidth,geometry::OutlineWidthThin).unwrap()
}

/// Filled select - shaded container, no border.
pub fn select_filled() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SELECT_FILLED))
		.with_token(common_props::BackgroundColor,colors::SurfaceContainerHighest).unwrap()
}

/// Text select - lowest emphasis.
pub fn select_text() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SELECT_TEXT))
		.with_token(common_props::BorderColorProp,colors::OutlineVariant).unwrap()
}

/// Focused select - primary-colored border. Compound selector `.select:focus`.
pub fn select_focus() -> Rule {
	Rule::new()
		.with_selector(Selector::AllOf(vec![
			Selector::class(SELECT),
			Selector::state(ElementState::Focused),
		]))
		.with_token(common_props::BorderColorProp,colors::Primary).unwrap()
}

/// The open select's floating option panel: absolutely positioned just below
/// the control (its positioned `.select` is the containing block), stretched
/// to its width, and lifted above subsequent content.
pub fn select_dropdown() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SELECT_DROPDOWN))
		.with_value(common_props::PositionProp, Position::Absolute)
		.with_value(common_props::InsetTop, Length::Percent(100.))
		.with_value(common_props::InsetLeft, Length::Rem(0.))
		.with_value(common_props::InsetRight, Length::Rem(0.))
		.with_value(common_props::ZIndexProp, 1000)
		.with_token(common_props::BackgroundColor,colors::SurfaceContainerHigh).unwrap()
		.with_token(common_props::BorderColorProp,colors::Outline).unwrap()
		.with_token(common_props::OutlineWidth,geometry::OutlineWidthThin).unwrap()
}

/// A `.select-dropdown` row.
pub fn select_option() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SELECT_OPTION))
		.with_value(common_props::DisplayProp, Display::Block)
		.with_value(common_props::Padding, Spacing {
			left: Length::Rem(1.),
			right: Length::Rem(1.),
			..Spacing::DEFAULT
		})
}

/// The active dropdown row (keyboard `:focus` or pointer `:hover`), inverted
/// to the primary role for prominence.
pub fn select_option_active() -> Rule {
	let active = |state: ElementState| {
		Selector::AllOf(vec![
			Selector::class(SELECT_OPTION),
			Selector::state(state),
		])
	};
	Rule::new()
		.with_selector(Selector::AnyOf(vec![
			active(ElementState::Focused),
			active(ElementState::Hovered),
		]))
		.with_token(ColorRoleProps,colors::PrimaryRole).unwrap()
}

/// The currently selected dropdown row, bold so it reads at a glance.
pub fn select_option_selected() -> Rule {
	Rule::new()
		.with_selector(Selector::AllOf(vec![
			Selector::class(SELECT_OPTION),
			Selector::state(ElementState::Selected),
		]))
		.with_token(common_props::FontWeightProp,typography::WeightBold).unwrap()
}

/// Error message text, ie validation feedback below an input.
pub fn error_text() -> Rule {
	Rule::new()
		.with_selector(Selector::class(ERROR_TEXT))
		.with_token(common_props::ForegroundColor,colors::Error).unwrap()
		.with_token(TypographyProps,typography::BodySmall).unwrap()
}

/// An error box: the [`Error`](crate::prelude::Error) container painted on the
/// material `Error`/`OnError` pair, for surfacing a failure inline (eg a render
/// error beside a broken widget).
pub fn error() -> Rule {
	Rule::new()
		.with_selector(Selector::class(ERROR))
		.with_token(common_props::BackgroundColor,colors::Error).unwrap()
		.with_token(common_props::ForegroundColor,colors::OnError).unwrap()
		.with_token(TypographyProps,typography::BodySmall).unwrap()
}
