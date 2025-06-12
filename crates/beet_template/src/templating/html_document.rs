use super::*;
use beet_common::prelude::*;
use bevy::prelude::*;
use sweet::prelude::HierarchyQueryExtExt;


/// Add this node to any bundle to have it rearranged into a valid HTML document structure.
/// The resulting structure is guaranteed to have the following layout:
/// ```text
/// (FragmentNode, HtmlDocumet)
/// ├─ DoctypeNode
/// ├─ (ElementNode, NodeTag(html))
/// 	 ├─ (ElementNode, NodeTag(head))
/// 	 ├─ (ElementNode, NodeTag(body))
/// ```
/// The contents of head and body are determined by performing checks on
/// the components and children of the [`HtmlDocument`] entity.
/// For instance any head or body elements are hoisted to the correct position.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(FragmentNode)]
pub struct HtmlDocument;

impl HtmlDocument {
	pub fn parse_bundle(bundle: impl Bundle) -> String {
		// add the bundle as a child to make rearranging easier
		HtmlFragment::parse_bundle((HtmlDocument, children![bundle]))
	}
}


pub(super) fn rearrange_html_document(
	mut commands: Commands,
	doctypes: Query<&DoctypeNode>,
	children: Query<&Children>,
	node_tags: Query<&NodeTag>,
	query: Populated<(Entity, &Children), Added<HtmlDocument>>,
) {
	for (doc_entity, doc_children) in query.iter() {
		let root = doc_children[0];

		let _doctype_node = match children
			.iter_descendants_inclusive(root)
			.find(|child| doctypes.contains(*child))
		{
			Some(doctype_node) => {
				commands.entity(doctype_node).insert(ChildOf(doc_entity));
				doctype_node
			}
			None => commands.spawn((DoctypeNode, ChildOf(doc_entity))).id(),
		};


		let html_node =
			match children.iter_descendants_inclusive(root).find(|child| {
				node_tags.get(*child).map_or(false, |tag| tag.0 == "html")
			}) {
				Some(html_node) => {
					commands.entity(html_node).insert(ChildOf(doc_entity));
					html_node
				}
				None => commands
					.spawn((
						ElementNode::open(),
						NodeTag::new("html"),
						ChildOf(doc_entity),
					))
					.id(),
			};

		let head_node =
			match children.iter_descendants_inclusive(root).find(|child| {
				node_tags.get(*child).map_or(false, |tag| tag.0 == "head")
			}) {
				Some(head_node) => {
					commands.entity(head_node).insert(ChildOf(html_node));
					head_node
				}
				None => commands
					.spawn((
						ElementNode::open(),
						NodeTag::new("head"),
						ChildOf(html_node),
					))
					.id(),
			};
		let _body_node =
			match children.iter_descendants_inclusive(root).find(|child| {
				node_tags.get(*child).map_or(false, |tag| tag.0 == "body")
			}) {
				Some(body_node) => {
					commands.entity(body_node).insert(ChildOf(html_node));
					body_node
				}
				None => commands
					.spawn((
						ElementNode::open(),
						NodeTag::new("body"),
						ChildOf(html_node),
					))
					.id(),
			};


		for node in children.iter_descendants_inclusive(root) {
			match node_tags.get(node) {
				Ok(tag)
					if matches!(
						tag.0.as_str(),
						"title" | "meta" | "link" | "style" | "script" | "base"
					) =>
				{
					commands.entity(node).insert(ChildOf(head_node));
				}
				_ => {}
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn text() {
		HtmlDocument::parse_bundle(rsx! {hello world})
			.xpect()
			.to_be(
				"<!DOCTYPE html><html><head></head><body>hello world</body></html>",
			);
	}
	#[test]
	fn elements() {
		HtmlDocument::parse_bundle(rsx! {<br/>}).xpect().to_be(
			"<!DOCTYPE html><html><head></head><body><br/></body></html>",
		);
	}
	#[test]
	fn fragment() {
		HtmlDocument::parse_bundle(rsx! {<br/><br/>}).xpect().to_be(
			"<!DOCTYPE html><html><head></head><body><br/><br/></body></html>",
		);
	}
	#[test]
	fn fragment_with_head() {
		HtmlDocument::parse_bundle(rsx! {<head><br/></head><br/>})
			.xpect()
			.to_be(
				"<!DOCTYPE html><html><head><br/></head><body><br/></body></html>",
			);
	}
	#[test]
	fn fragment_with_head_and_body() {
		HtmlDocument::parse_bundle(
			rsx! {<body><br/></body><!doctype pizza><head>7</head>},
		)
		.xpect()
		.to_be("<!doctype pizza><html><head>7</head><body><br/></body></html>");
	}
	#[test]
	fn html_element() {
		HtmlDocument::parse_bundle(rsx! {<html><br/></html>})
			.xpect()
			.to_be(
				"<!DOCTYPE html><html><head></head><body><br/></body></html>",
			);
	}
}
