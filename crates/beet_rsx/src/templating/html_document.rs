use super::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;


/// Added to the *top-level* [`InstanceRoot`], and useful for getting the document root,
/// rather than the root of a single macro.
///
/// When recursing [`Children`] or [`TemplateRoot`] it is reccomended to use this type as the
/// starting point, its very easy to create unexpected behavior by starting at something like [`SnippetRoot`],
/// which appears all over the place.
///
/// Add this node to any bundle to have it rearranged into a valid HTML document structure.
/// The resulting structure is guaranteed to have the following layout:
/// ```text
/// (FragmentNode, HtmlDocumet)
/// ├─ DoctypeNode
/// ├─ (ElementNode, NodeTag(html))
/// 	 ├─ (ElementNode, NodeTag(head))
/// 	 ├─ (ElementNode, NodeTag(body))
/// ```
/// Only a single check is performed: whether the document *contains* a `<!DOCTYPE html>` tag
/// anywhere in the document.
/// - If this is is the case, no rearrangement is done.
/// - Otherwise, a new document structure is created with the root moved into the body.
/// This will create malformed html for partially correct documents, so it is the user's responsibility
/// to either pass a valid document structure or none at all.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(InstanceRoot, BeetRoot)]
pub struct HtmlDocument;

impl HtmlDocument {
	pub fn parse_bundle(bundle: impl Bundle) -> String {
		// add the bundle as a child to make rearranging easier
		HtmlFragment::parse_bundle((HtmlDocument, bundle))
	}
}

/// see [`HtmlDocument`] for rules on how this is rearranged
pub(super) fn rearrange_html_document(
	mut commands: Commands,
	doctypes: Query<&DoctypeNode>,
	children: Query<&Children>,
	query: Populated<(Entity, Option<&Children>), Added<HtmlDocument>>,
) {
	for (doc_entity, doc_children) in query.iter() {
		if children
			.iter_descendants_inclusive(doc_entity)
			.any(|child| doctypes.contains(child))
		{
			// doctype found, assume correct document structure
			continue;
		}
		// no doctype found, create full document structure and
		// move the children into the body
		commands.entity(doc_entity).with_children(|parent| {
			parent.spawn(DoctypeNode);
			parent
				.spawn((ElementNode::open(), NodeTag::new("html")))
				.with_children(|parent| {
					parent.spawn((ElementNode::open(), NodeTag::new("head")));
					let mut body = parent
						.spawn((ElementNode::open(), NodeTag::new("body")));
					if let Some(children) = doc_children {
						body.add_children(children);
					}
				});
		});
	}
}

/// Elements with either [`HtmlHoistDirective`] or a [`HtmlConstants::hoist_to_head_tags`]
/// tag will be hoisted to their respective part of the document.
pub(super) fn hoist_document_elements(
	mut commands: Commands,
	constants: Res<HtmlConstants>,
	documents: Populated<(Entity, Option<&SnippetRoot>), Added<HtmlDocument>>,
	children: Query<&Children>,
	node_tags: Query<&NodeTag>,
	directives: Query<&HtmlHoistDirective>,
) -> Result {
	for (document, idx) in documents.iter() {
		let get_idx = || {
			idx.map(|idx| idx.to_string())
				.unwrap_or_else(|| String::from("unknown location"))
		};

		let head = children
			.iter_descendants(document)
			.find(|child| {
				node_tags.get(*child).map_or(false, |tag| tag.0 == "head")
			})
			.ok_or_else(|| {
				let idx = get_idx();
				// commands.run_system_cached_with(output_info, document);
				bevyhow!(
					"Invalid HTML document: no head tag found\nlocation: {idx}"
				)
			})?;
		let body = children
			.iter_descendants(document)
			.find(|child| {
				node_tags.get(*child).map_or(false, |tag| tag.0 == "body")
			})
			.ok_or_else(|| {
				let idx = get_idx();
				// commands.run_system_cached_with(output_info, document);
				bevyhow!(
					"Invalid HTML document: no body tag found\nlocation: {idx}"
				)
			})?;
		for entity in children.iter_descendants(document) {
			match (directives.get(entity), node_tags.get(entity)) {
				(Ok(HtmlHoistDirective::Head), _) => {
					commands.entity(head).add_child(entity);
				}
				(Ok(HtmlHoistDirective::Body), _) => {
					commands.entity(body).add_child(entity);
				}
				(Ok(HtmlHoistDirective::None), _) => {
					// leave in place, even if matches a hoist_to_head_tag
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
	Ok(())
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
			// will be hoisted to correct location
			.with_child(event_playback_script(&html_constants))
			.with_child(load_wasm_script(&html_constants));
	}
}

fn event_playback_script(html_constants: &HtmlConstants) -> impl Bundle {
	script(format!(
		r#"
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
		init({{ module_or_path: '{bin_path}' }})
			.catch((error) => {{
				if (!error.message.startsWith("Using exceptions for control flow,"))
					throw error
		}})
"#,
		js_path = html_constants.wasm_js_url(),
		bin_path = html_constants.wasm_bin_url()
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
			Attributes[(AttributeKey::new("type"), TextNode::new("module"),)]
		),
		children![TextNode::new(content.into())],
	)
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn text() {
		HtmlDocument::parse_bundle(rsx! { hello world }).xpect_str(
			"<!DOCTYPE html><html><head></head><body>hello world</body></html>",
		);
	}
	#[test]
	fn elements() {
		HtmlDocument::parse_bundle(rsx! { <br /> }).xpect_str(
			"<!DOCTYPE html><html><head></head><body><br/></body></html>",
		);
	}
	#[test]
	fn fragment() {
		HtmlDocument::parse_bundle(rsx! {
			<br />
			<br />
		})
		.xpect_str(
			"<!DOCTYPE html><html><head></head><body><br/><br/></body></html>",
		);
	}
	#[test]
	fn empty_fragment() {
		HtmlDocument::parse_bundle(rsx! {</>}).xpect_str(
			"<!DOCTYPE html><html><head></head><body></body></html>",
		);
	}
	#[test]
	fn ignores_incomplete() {
		HtmlDocument::parse_bundle(rsx! {
			<head>
				<br />
			</head>
			<br />
		})
		.xpect_str(
			"<!DOCTYPE html><html><head></head><body><head><br/></head><br/></body></html>",
		);
	}
	#[test]
	#[ignore = "noisy"]
	#[should_panic(expected = "Invalid HTML document: no body tag found")]
	fn partial() {
		HtmlDocument::parse_bundle(rsx! {
			<!DOCTYPE html>
			<html>
				<head>
					<br />
				</head>
			</html>
		})
		.xpect_str(
			"<!DOCTYPE html><html><head><br/></head><body></body></html>",
		);
	}

	#[test]
	#[cfg(feature = "css")]
	fn hoist_style_tag() {
		HtmlDocument::parse_bundle(rsx! { <style>foo{}</style> })
			.xpect_snapshot();
	}
	#[test]
	fn hoist_script_tag() {
		HtmlDocument::parse_bundle(rsx! {
			<script></script>
			<br />
		})
		.xpect_snapshot();
	}
	#[test]
	fn hoist_top_tag() {
		HtmlDocument::parse_bundle(rsx! {
			<script />
			<!DOCTYPE html>
			<html>
				<head></head>
				<body></body>
			</html>
		})
		.xpect_str(
			"<!DOCTYPE html><html><head><script></script></head><body></body></html>",
		);
	}
	#[test]
	fn hoist_directive() {
		HtmlDocument::parse_bundle(
		rsx! {
			<!DOCTYPE html>
			<html>
				<head>
					<br hoist:body />
				</head>
				<body>
					<span hoist:head />
					<script hoist:none />
				</body>
			</html>
		},
	)
	.xpect_str(
		"<!DOCTYPE html><html><head><span/></head><body><script></script><br/></body></html>",
	);
	}
	#[test]
	fn hydration_scripts() {
		HtmlDocument::parse_bundle(rsx! {<div client:load>}).xpect_snapshot();
	}
}
