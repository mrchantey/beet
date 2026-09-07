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
