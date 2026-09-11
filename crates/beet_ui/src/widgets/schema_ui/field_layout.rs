//! The shapes every generated control is built from: the label row a leaf
//! wears, the disclosure a composite's rows sit in, the lift of a constructed
//! widget into child position, and the child path a nested control binds.
//!
//! Neutral between the arms ([`scalar_field`](super::scalar_field),
//! [`composite_field`](super::composite_field),
//! [`dependent_field`](super::dependent_field)), so none of them reaches into
//! another for a shape they all share.
use crate::prelude::*;
use beet_core::prelude::*;

/// Wrap a control in a `<label>` row (the key above its value, per the form
/// rules), or pass it through unlabelled.
pub(super) fn labeled<M>(
	label: Option<String>,
	widget: impl IntoSnippet<M>,
) -> Snippet {
	match label {
		Some(label) => rsx! { <label>{label}{widget}</label> }.any_snippet(),
		None => Snippet::from_bundle(widget.into_snippet()),
	}
}

/// The label a named field wears: its own hint verbatim, else its key made
/// readable.
///
/// A key is an identifier and reads like one — a form that asks for
/// `allow_additional` and `min_items` is showing its wiring. The schema's own
/// `label` still wins untouched, since an author who named a field meant it.
///
/// Deliberately unmarked for `required`: a field is required by default, so the
/// conventional `*` would land on almost every label and distinguish nothing.
/// The one place it matters — a commit that would leave existing rows invalid —
/// names the field in the editor's error line.
pub(super) fn field_label(named: &NamedFieldSchema) -> String {
	named
		.label
		.as_ref()
		.map(|label| label.to_string())
		.unwrap_or_else(|| humanize(&named.key))
}

/// An identifier as a person reads it: `on_missing` becomes `On missing`.
///
/// Sentence case rather than title case, because a key is a phrase (`allow
/// additional`), not a heading, and capitalising every word reads as a menu.
pub(super) fn humanize(key: &str) -> String {
	let spaced = key.replace(['_', '-'], " ");
	let mut chars = spaced.chars();
	match chars.next() {
		Some(first) => first.to_uppercase().chain(chars).collect(),
		None => spaced,
	}
}

/// Pair a generated field with its schema's description, rendered as help text
/// under the control.
///
/// A description is the one thing a schema says *to the person filling the form*
/// and nothing rendered it, so every hint an author wrote was invisible. The
/// pair rides its own tight column so the form's row gap separates fields rather
/// than a field from its own help.
pub(super) fn hinted(hint: Option<&str>, field: Snippet) -> Snippet {
	match hint {
		None => field,
		Some(hint) => {
			let hint = hint.to_string();
			rsx! {
				<div {hint_column()}>
					{field}
					<small {hint_style()}>{hint}</small>
				</div>
			}
			.any_snippet()
		}
	}
}

/// The field-plus-help column: tight, so the help sits against its control.
fn hint_column() -> impl Bundle {
	inline_class![
		(style::common_props::DisplayProp, style::Display::Flex),
		(
			style::common_props::FlexDirectionProp,
			style::Direction::Vertical
		),
		(
			style::common_props::AlignItemsProp,
			style::AlignItems::Stretch
		),
	]
}

/// Help text: the muted, smaller voice a hint speaks in, so it reads as
/// guidance rather than as another field.
fn hint_style() -> impl Bundle {
	inline_class![
		Declaration::token(
			style::common_props::ForegroundColor,
			style::material::colors::OnSurfaceVariant
		),
		Declaration::token(
			style::common_props::FontSize,
			style::material::typography::FontSizeBodySmall
		),
	]
}

/// Wrap a composite's rows in a titled section, or pass them through when there
/// is no title (the form's own top level, which is already the group).
///
/// A heading rather than a disclosure: a generated form is a place to *fill in*
/// fields, so hiding them behind a caret buys nothing, and the heading carries
/// the nesting a reader needs. A collapsible group is the app's call, which
/// `ToggleSchemaEditor` makes by authoring its own `<details>`.
pub(super) fn group<M>(
	title: Option<String>,
	rows: impl IntoSnippet<M>,
) -> Snippet {
	match title {
		Some(title) => rsx! {
			<section>
				<h3 {group_title_style()}>{title}</h3>
				{rows}
			</section>
		}
		.any_snippet(),
		None => Snippet::from_bundle(rows.into_snippet()),
	}
}

/// A field group's title reads as a label, not as prose chrome.
///
/// The shipped heading steps put `<h1>`..`<h4>` above `1em`, which the charcell
/// font renders fullwidth — right for a page's own headings, wrong for a group
/// of inputs, where a poster-set `Ｉｔｅｍ　１` shouts over the fields it names.
/// Body size at the heading's weight keeps the hierarchy without the scale.
///
/// The prose margin goes with it: `h1`-`h6` carry a `1rem` bottom gap for
/// paragraphs to breathe under, which on a terminal is a whole blank row
/// between a group's title and its first field. A title belongs *to* the
/// fields under it, so it sits against them.
fn group_title_style() -> impl Bundle {
	inline_class![
		Declaration::token(
			style::common_props::FontSize,
			style::material::typography::FontSizeBodyLarge
		),
		Declaration::token(
			style::common_props::LineHeight,
			style::material::typography::LineHeightBodyLarge
		),
		(style::common_props::MarginProp, style::Spacing::DEFAULT),
	]
}

/// What an empty collection shows in place of its rows.
///
/// An empty list otherwise renders as *nothing*: a heading, a gap, and an add
/// button floating in space, which reads as a broken widget rather than an
/// empty one. Muted and italic, because it is the absence of content and must
/// never be mistaken for a value.
pub(super) fn empty_note(text: impl Into<String>) -> Snippet {
	let text = text.into();
	rsx! { <div {empty_note_style()}>{text}</div> }.any_snippet()
}

/// The empty note's voice.
fn empty_note_style() -> impl Bundle {
	inline_class![
		Declaration::token(
			style::common_props::ForegroundColor,
			style::material::colors::OnSurfaceVariant
		),
		(style::common_props::FontStyleProp, style::FontStyle::Italic),
	]
}

/// Lift a constructed widget into a child-position [`Snippet`], the
/// struct-literal twin of an `rsx!` `<Widget/>` tag — reached for here because a
/// dispatched arm passes `Option`s straight into optional props, which a tag
/// cannot express.
pub(super) fn widget(template: impl BuildTemplate) -> Snippet {
	Snippet::from_bundle(template.into_snippet_bundle())
}

/// A child position of `field`: the same document, one segment deeper.
pub(super) fn child_field(
	field: &FieldRef,
	segment: impl Into<FieldSegment>,
) -> FieldRef {
	FieldRef {
		document: field.document.clone(),
		field_path: field.field_path.with_pushed(segment),
		on_missing: default(),
	}
}
