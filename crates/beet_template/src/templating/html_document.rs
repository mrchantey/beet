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
/// Only a single check is performed: whether the root is a fragment containing an `<html>` tag.
/// - If this is is the case, missing nodes are inserted.
/// - Otherwise, a new document structure is created with the root moved into the body.
/// This will create malformed html for partially correct documents, so it is the user's responsibility
/// to either pass a valid document structure or none at all.
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

/// Rarrange the HTML document accountinf for one of two cases:
/// 1. The root is a fragment containing a <html> tag. doctype, head and body are added if missing.
/// 2. All other cases: root is moved to the body.
pub(super) fn rearrange_html_document(
	mut commands: Commands,
	doctypes: Query<&DoctypeNode>,
	children: Query<&Children>,
	node_tags: Query<&NodeTag>,
	query: Populated<(Entity, &Children), Added<HtmlDocument>>,
) {
	for (doc_entity, doc_children) in query.iter() {
		let root = doc_children[0];
		if let Some(html_node) =
			children.iter_direct_descendants(root).find(|child| {
				node_tags.get(*child).map_or(false, |tag| tag.0 == "html")
			}) {
			// a html tag was found, add missing nodes
			if !children
				.iter_direct_descendants(root)
				.any(|child| doctypes.contains(child))
			{
				// ensure the doctype is the first child
				let mut new_children = vec![commands.spawn(DoctypeNode).id()];
				new_children.extend(children.iter_direct_descendants(root));
				commands.entity(root).replace_children(&new_children);
			}
			if !children.iter_direct_descendants(html_node).any(|child| {
				node_tags.get(child).map_or(false, |tag| tag.0 == "head")
			}) {
				commands
					.entity(html_node)
					.with_child((ElementNode::open(), NodeTag::new("head")));
			}
			if !children.iter_direct_descendants(html_node).any(|child| {
				node_tags.get(child).map_or(false, |tag| tag.0 == "body")
			}) {
				commands
					.entity(html_node)
					.with_child((ElementNode::open(), NodeTag::new("body")));
			}
		} else {
			// no html tag found, create full document structure and
			// move the root into the body
			let new_root = commands
				.spawn(FragmentNode)
				.with_children(|parent| {
					parent.spawn((DoctypeNode,));
					parent
						.spawn((ElementNode::open(), NodeTag::new("html")))
						.with_children(|parent| {
							parent.spawn((
								ElementNode::open(),
								NodeTag::new("head"),
							));
							parent
								.spawn((
									ElementNode::open(),
									NodeTag::new("body"),
								))
								.add_child(root);
						});
				})
				.id();
			commands.entity(doc_entity).replace_children(&[new_root]);
		}
	}
}

/// Elements with either [`HtmlHoistDirective`] or a [`HtmlConstants::hoist_to_head_tags`]
/// tag will be hoisted to their respective part of the document.
pub(super) fn hoist_document_elements(
	mut commands: Commands,
	constants: Res<HtmlConstants>,
	documents: Populated<Entity, Added<HtmlDocument>>,
	children: Query<&Children>,
	node_tags: Query<&NodeTag>,
	directives: Query<&HtmlHoistDirective>,
) {
	for document in documents.iter() {
		let head = children
			.iter_descendants(document)
			.find(|child| {
				node_tags.get(*child).map_or(false, |tag| tag.0 == "head")
			})
			.expect("Invalid HTML document: no head tag found");
		let body = children
			.iter_descendants(document)
			.find(|child| {
				node_tags.get(*child).map_or(false, |tag| tag.0 == "body")
			})
			.expect("Invalid HTML document: no body tag found");
		for entity in children.iter_descendants(document) {
			match (directives.get(entity), node_tags.get(entity)) {
				(Ok(HtmlHoistDirective::Head), _) => {
					commands.entity(head).add_child(entity);
				}
				(Ok(HtmlHoistDirective::Body), _) => {
					commands.entity(body).add_child(entity);
				}
				(Ok(HtmlHoistDirective::None), _) => {
					// leave in place
				}
				(Err(_), Ok(tag))
					if constants.hoist_to_head_tags.contains(&tag.0) =>
				{
					commands.entity(head).add_child(entity);
				}
				(Err(_), _) => {
					// leave in place
				}
			}
		}
	}
}

/// Any tree containing a [`ClientLoadDirective`] or [`ClientOnlyDirective`]
/// will need several scripts to be ran in order to hydrate the page.
pub(super) fn insert_hydration_scripts(
	mut commands: Commands,
	html_constants: Res<HtmlConstants>,
	children: Query<&Children>,
	is_hydrated: Query<
		(),
		Or<(With<ClientLoadDirective>, With<ClientOnlyDirective>)>,
	>,
	documents: Populated<Entity, Added<HtmlDocument>>,
) {
	for doc in documents.iter().filter(|doc| {
		children
			.iter_descendants(*doc)
			.any(|child| is_hydrated.contains(child))
	}) {
		commands
			.entity(doc)
			.with_child(event_playback_script(&html_constants))
			.with_child(load_wasm_script(&html_constants));
	}
}

fn event_playback_script(html_constants: &HtmlConstants) -> impl Bundle {
	script(format!(
		r#"
// console.log('sweet has loaded')
globalThis.{event_store} = []
globalThis.{event_handler} = (id,event) => globalThis.{event_store}.push([id, event])
"#,
		event_store = html_constants.event_store,
		event_handler = html_constants.event_handler,
	))
}

fn load_wasm_script(html_constants: &HtmlConstants) -> impl Bundle {
	script(format!(
		r#"
		import init from '{js_path}'
		init('{bin_path}')
			.catch((error) => {{
				if (!error.message.startsWith("Using exceptions for control flow,"))
					throw error
		}})
"#,
		js_path = html_constants.wasm_js_path,
		bin_path = html_constants.wasm_bin_path
	))
}

// fn insert_tree_location_map(&self, node: &WebNode, doc: &mut HtmlDocument) {
// 	let loc_map = node.xpipe(NodeToTreeLocationMap);
// 	let loc_map =
// 		ron::ser::to_string_pretty(&loc_map, Default::default()).unwrap();
// 	let el = HtmlElementNode::inline_script(loc_map, vec![
// 		HtmlAttribute {
// 			key: "type".to_string(),
// 			value: Some("beet/ron".to_string()),
// 		},
// 		HtmlAttribute {
// 			key: self.html_constants.loc_map_key.to_string(),
// 			value: None,
// 		},
// 	]);
// 	doc.body.push(el.into());
// }


fn script(content: impl Into<String>) -> impl Bundle {
	(
		ElementNode::open(),
		NodeTag::new("script"),
		related!(
			Attributes[(
				AttributeKeyStr::new("type"),
				AttributeValueStr::new("module"),
			)]
		),
		children![TextNode::new(content)],
	)
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
	fn malformed() {
		HtmlDocument::parse_bundle(rsx! {<head><br/></head><br/>})
			.xpect()
			.to_be(
				"<!DOCTYPE html><html><head></head><body><head><br/></head><br/></body></html>",
			);
	}
	#[test]
	fn partial() {
		HtmlDocument::parse_bundle(
			rsx! {<!DOCTYPE html><html><head><br/></head></html>},
		)
		.xpect()
		.to_be("<!DOCTYPE html><html><head><br/></head><body></body></html>");
	}
	#[test]
	fn hoist_tag() {
		HtmlDocument::parse_bundle(
			rsx! {<script></script><br/>},
		)
		.xpect()
		.to_be("<!DOCTYPE html><html><head><script></script></head><body><br/></body></html>");
	}
	#[test]
	fn hoist_directive() {
		HtmlDocument::parse_bundle(
		rsx! {<!DOCTYPE html><html><head><br hoist:body/></head><body><span hoist:head/><script hoist:none/></body></html>},
	)
	.xpect()
	.to_be(
		"<!DOCTYPE html><html><head><span/></head><body><script/><br/></body></html>",
	);
	}
	#[test]
	fn hydration_scripts() {
		HtmlDocument::parse_bundle(
		rsx! {<div client:load>},
	)
	.xpect()
	.to_be(
		"<!DOCTYPE html><html><head><script type=\"module\">\n// console.log('sweet has loaded')\nglobalThis._beet_event_store = []\nglobalThis._beet_event_handler = (id,event) => globalThis._beet_event_store.push([id, event])\n</script><script type=\"module\">\n\t\timport init from '/wasm/main.js'\n\t\tinit('/wasm/main_bg.wasm')\n\t\t\t.catch((error) => {\n\t\t\t\tif (!error.message.startsWith(\"Using exceptions for control flow,\"))\n\t\t\t\t\tthrow error\n\t\t})\n</script></head><body><div/></body></html>",
	);
	}
}
