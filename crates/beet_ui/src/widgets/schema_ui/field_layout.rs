//! The shapes every generated control is built from: the label row a leaf
//! wears, the disclosure a composite's rows sit in, the lift of a constructed
//! widget into child position, and the child path a nested control binds.
//!
//! Neutral between the arms ([`scalar_field`](super::scalar_field),
//! [`composite_field`](super::composite_field),
//! [`dependent_field`](super::dependent_field)), so none of them reaches into
//! another for a shape they all share.
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

/// Wrap a composite's rows in a titled disclosure, or pass them through when
/// there is no title (the form's own top level, which is already the group).
pub(super) fn group<M>(
	title: Option<String>,
	rows: impl IntoSnippet<M>,
) -> Snippet {
	match title {
		Some(title) => rsx! {
			<details open>
				<summary>{title}</summary>
				{rows}
			</details>
		}
		.any_snippet(),
		None => Snippet::from_bundle(rows.into_snippet()),
	}
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
