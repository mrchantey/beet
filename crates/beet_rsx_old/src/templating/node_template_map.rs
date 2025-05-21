use crate::prelude::*;
#[allow(unused)]
use anyhow::Result;
use beet_common::prelude::*;
use rapidhash::RapidHashMap;
use sweet::prelude::WorkspacePathBuf;

/// The beet cli will visit all files in a crate and collect each
/// rsx! macro into a ron structure of this type. Because it operates on the level
/// of *tokens* it is actually unaware of this type directly and instead
/// generates it in a [`ron`] format via [beet_rsx_parser::RstmlToRsxTemplate].
/// The advantage of this approach is we can build templates statically without
/// a compile step.
///
/// When joining an [WebNodeTemplate] with an [RustyPartMap],
/// we need the entire [NodeTemplateMap] to resolve components.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeTemplateMap {
	/// The root directory used to create the templates, templates
	/// with a location outside of this root will not be expected to exists and
	/// so will not produce an error.
	// canonicalized [here](crates/beet_router/src/parser/build_template_map/mod.rs#L110-L111)
	pub root: WorkspacePathBuf,
	/// Template for each node with a [`TemplateDirective::NodeTemplate`], keyed by their span.
	pub templates: RapidHashMap<FileSpan, WebNodeTemplate>,
}

impl std::ops::Deref for NodeTemplateMap {
	type Target = RapidHashMap<FileSpan, WebNodeTemplate>;
	fn deref(&self) -> &Self::Target { &self.templates }
}

// TODO use a visitor that doesnt exit early if a parent has no nodes.
// has no location, it may have a child that does and so should be templated.
/// Find a matching template for the given [`WebNode`] and apply it, returning the
/// updated root.
/// If the root has no location or the location is outside the templates root directory,
/// the root is returned unchanged.
///
/// ## Errors
/// If the root is inside the templates root directory and a template was not found.
impl Pipeline<WebNode, TemplateResult<WebNode>> for &NodeTemplateMap {
	fn apply(self, mut node: WebNode) -> TemplateResult<WebNode> {
		let mut result = Ok(());
		// templates cannot track block initials and component nodes,
		// so we certainly must visit them
		// even though they can track element and slot children, we still need to visit
		// because the parent may not have a location/template, in most of these cases
		// the child will not have a location so itll be a noop.
		VisitWebNodeMut::walk(&mut node, |node| {
			// this will actually mutate the node, effecting which children get visited next.
			// this makes it vunerable to a stack overflow, for instance if the location
			// was set on a child instead of the root, the child would be replaced
			// with the root and its child would be visited again.
			if let Err(err) = self.apply_template(node) {
				result = Err(err);
			}
		});

		result.map(|_| node)
	}
}

impl NodeTemplateMap {
	pub fn new(
		root: WorkspacePathBuf,
		templates: Vec<WebNodeTemplate>,
	) -> Self {
		Self {
			root,
			templates: templates
				.into_iter()
				.map(|template| (template.span().clone(), template))
				.collect(),
		}
	}

	pub fn root(&self) -> &WorkspacePathBuf { &self.root }

	/// Load the template map created by the beet cli.
	/// Load the template map created by the beet cli.
	#[cfg(all(feature = "serde", not(target_arch = "wasm32")))]
	pub fn load(src: impl AsRef<std::path::Path>) -> Result<Self> {
		let tokens = sweet::prelude::ReadFile::to_string(src)?;
		let this: Self = ron::de::from_str(&tokens.to_string())?;
		Result::Ok(this)
	}

	fn apply_template(&self, node: &mut WebNode) -> TemplateResult<()> {
		if !node.is_template() {
			return Ok(());
		}
		let span = node.span().clone();

		if let Some(template) = self.templates.get(&span) {
			// println!("applying template to node: {span}");
			// clone because multiple nodes may have the same location
			*node = (std::mem::take(node), template.clone())
				.xpipe(ApplyTemplateToNode)
				.map_err(|err| err.with_location(span.clone()))?;

			return Ok(());
		}
		// we cant check on wasm
		#[cfg(target_arch = "wasm32")]
		return Ok(());

		#[cfg(not(target_arch = "wasm32"))]
		{
			let root_abs = self.root.into_abs().unwrap_or_default();
			if span
				.file()
				.into_abs()
				.map(|p| p.starts_with(&root_abs))
				.unwrap_or(true)
			{
				Err(TemplateError::NoTemplate {
					received: self
						.templates
						.keys()
						.map(|x| x.clone())
						.collect(),
					expected: span.clone(),
				}
				.with_location(span.clone()))
			} else {
				// if the node location is outside the templates root directory,
				// it wouldn't be expected to have a template.
				println!(
					"web node is outside templates dir so no template will be applied:\n{}",
					span
				);
				Ok(())
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	/// used for testing, load directly from a collection of template roots.
	#[cfg(test)]
	pub fn test_template_map(
		templates: Vec<WebNodeTemplate>,
	) -> NodeTemplateMap {
		NodeTemplateMap {
			root: WorkspacePathBuf::new(file!()),
			templates: templates
				.into_iter()
				.filter_map(|node| match node.is_template() {
					true => Some((node.span().clone(), node)),
					false => None,
				})
				.collect(),
		}
	}

	#[derive(Default, Node)]
	struct MyComponent {
		value: usize,
	}
	fn my_component(props: MyComponent) -> WebNode {
		rsx! { <div>the value is {props.value}<slot /></div> }
	}

	// test a roundtrip split/join,
	#[test]
	fn split_join() {
		let some_val = 3;

		let node1 = rsx! {
			<div key str="value" num=32 ident=some_val>
				hello world
			</div>
		};

		let html1 = node1.clone().xpipe(RsxToHtmlString::default()).unwrap();
		let template = node1.xref().xpipe(NodeToTemplate).unwrap();
		let map = test_template_map(vec![template]);
		let html2 = node1
			.xpipe(&map)
			.unwrap()
			.xpipe(RsxToHtmlString::default())
			.unwrap();
		expect(html1).to_be(html2);
	}
	#[test]
	fn rsx_template_match_simple() {
		let some_val = 3;
		let node1 = rsx! {
			<div ident=some_val>
				<div ident=some_val />
			</div>
		};
		let node2_template = rsx_template! {
			<div ident=some_val>
				<div ident=some_val />
			</div>
		}
		.reset_spans_and_trackers();
		expect(&node1.xref().xpipe(NodeToTemplate).unwrap())
			.not()
			.to_be(&node2_template);

		expect(
			&node1
				.xref()
				.xpipe(NodeToTemplate)
				.unwrap()
				.reset_spans_and_trackers(),
		)
		.to_be(&node2_template);
	}
	#[test]
	fn rsx_template_match_complex() {
		let some_val = 3;

		let node1 = rsx! {
			<div key str="value" num=32 ident=some_val onclick=|_| {}>
				<p>
					hello <MyComponent style:cascade value=3>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};
		let node2_template = rsx_template! {
			<div key str="value" num=32 ident=some_val onclick=|_| {}>
				<p>
					hello <MyComponent style:cascade value=3>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		}
		.reset_spans_and_trackers();
		expect(&node1.xref().xpipe(NodeToTemplate).unwrap())
			.not()
			.to_be(&node2_template);

		node1
			.xref()
			.xpipe(NodeToTemplate)
			.unwrap()
			.reset_spans_and_trackers()
			.xpect()
			.to_be(node2_template);
	}
	#[test]
	#[ignore = "flaky and doesnt really test anything"]
	fn trackers_match() {
		let node1 = rsx! {
			<MyComponent style:cascade value=3>
				Hello
			</MyComponent>
		};
		let node2_template = rsx_template! {
			<MyComponent style:cascade value=3>
				Hello
			</MyComponent>
		};
		let WebNode::Component(RsxComponent {
			tracker: tracker1, ..
		}) = node1
		else {
			panic!();
		};
		let WebNodeTemplate::Component {
			tracker: tracker2, ..
		} = node2_template
		else {
			panic!();
		};
		expect(tracker1.tokens_hash).to_be(418568435446366402);
		expect(tracker2.tokens_hash).to_be(2714607922737053776);
	}

	#[test]
	fn nested_templates() {
		let node = rsx! {
			<div>
				<MyComponent value=3>
					<div>some child</div>
				</MyComponent>
			</div>
		};

		let template = node.xref().xpipe(NodeToTemplate).unwrap();
		// these templates are usually generated by statically looking at a file,
		// here we create one from a default MyComponent, so the value: 4 will
		// be ignored
		let my_component_template = MyComponent {
			value: 4,
			..Default::default()
		}
		.into_node()
		.xref()
		.xpipe(NodeToTemplate)
		.unwrap();
		let map = test_template_map(vec![template, my_component_template]);

		let html1 = node.clone().xpipe(RsxToHtmlString::default()).unwrap();
		let html2 = node
			.xpipe(&map)
			.unwrap()
			.xpipe(RsxToHtmlString::default())
			.unwrap();
		expect(&html1).to_be("<div><div data-beet-rsx-idx=\"3\">the value is 3<div>some child</div></div></div>");
		expect(html1).to_be(html2);
	}

	#[test]
	// cant canonicalize on wasm so cant check validity
	#[cfg(not(target_arch = "wasm32"))]
	fn ignores_exterior_roots() {
		let map = test_template_map(vec![]);
		let comp = rsx! { <div>foo</div> };

		// interior root, not found
		expect(comp.clone().xpipe(&map)).to_be_err();
		// exterior root, ok
		expect(
			comp.with_span(FileSpan::new_with_start(
				WorkspacePathBuf::new("../"),
				1,
				1,
			))
			.xpipe(&map),
		)
		.to_be_ok();
	}
}
