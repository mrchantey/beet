use crate::prelude::*;
#[allow(unused)]
use anyhow::Result;
use sweet::prelude::WorkspacePathBuf;



/// A cli router will visit all files in a crate and collect each
/// rsx! macro into a ron structure of this type using
/// [beet_rsx_parser::RstmlToRsxTemplate]. Because it operates on the level
/// of *tokens* and *ron format*, it is actually unaware of this type directly.
/// The advantage of this approach is we can build templates statically without
/// a compile step.
///
///
/// When joining an [RsxTemplateNode] with an [RustyPartMap],
/// we need the entire [RsxTemplateMap] to resolve components.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RsxTemplateMap {
	/// The canonicalized root directory used to create the templates, templates
	/// with a location outside of this root will not be expected to exists and
	/// so will not produce an error.
	// canonicalized [here](crates/beet_router/src/parser/build_template_map/mod.rs#L110-L111)
	root: WorkspacePathBuf,
	/// The templates themselves, keyed by their location.
	pub templates: HashMap<RsxMacroLocation, RsxTemplateNode>,
}



// TODO use a visitor that doesnt exit early if a parent has no nodes.
// has no location, it may have a child that does and so should be templated.
/// Find a matching template for the given [`RsxNode`] and apply it, returning the
/// updated root.
/// If the root has no location or the location is outside the templates root directory,
/// the root is returned unchanged.
///
/// ## Errors
/// If the root is inside the templates root directory and a template was not found.
impl RsxPipeline<RsxNode, TemplateResult<RsxNode>> for &RsxTemplateMap {
	fn apply(self, value: RsxNode) -> TemplateResult<RsxNode> {
		// RsxNode
		self.apply_template(value)
	}
}

impl RsxTemplateMap {
	pub fn root(&self) -> &WorkspacePathBuf { &self.root }

	/// Load the template map serialized by [beet_rsx_parser::RstmlToRsxTemplate]
	#[cfg(all(feature = "serde", not(target_arch = "wasm32")))]
	pub fn load(src: impl AsRef<std::path::Path>) -> Result<Self> {
		use sweet::prelude::ReadFile;
		{
			let tokens = ReadFile::to_string(src)?;
			let this: Self = ron::de::from_str(&tokens.to_string())?;
			Result::Ok(this)
		}
	}

	fn apply_template(&self, node: RsxNode) -> TemplateResult<RsxNode> {
		let Some(location) = node.location().cloned() else {
			// if it doesnt have a location, we dont even try to apply a template
			return Ok(node);
		};

		if let Some(template_root) = self.templates.get(&location) {
			let node = self
				.apply_template_for_node(
					template_root.clone(),
					&mut RustyPartMap::collect(node),
				)
				.map_err(|err| err.with_location(location.clone()))?;
			Ok(node)
		} else if location.file.starts_with(&self.root) {
			Err(TemplateError::NoTemplate {
				received: self.templates.keys().map(|x| x.clone()).collect(),
				expected: location.clone(),
			}
			.with_location(location.clone()))
		} else {
			// println!(
			// 	"rsx node is outside templates dir so no template will be applied:\n{:?}",
			// 	location
			// );
			Ok(node)
		}
	}

	// fn template_should_exist(&self, location: &RsxMacroLocation) -> bool {
	// 	location.file.starts
	// }


	/// Create an [`RsxNode`] from a template and hydrated nodes.
	fn apply_template_for_node(
		&self,
		node: RsxTemplateNode,
		rusty_map: &mut RustyPartMap,
	) -> TemplateResult<RsxNode> {
		// i dont like passing self like this, kind of hides recursion
		node.into_rsx_node(self, rusty_map)
	}
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;


	/// used for testing, load directly from a collection of template roots.
	#[cfg(test)]
	pub fn test_template_map(
		templates: Vec<RsxTemplateNode>,
	) -> RsxTemplateMap {
		RsxTemplateMap {
			root: WorkspacePathBuf::new(file!()),
			templates: templates
				.into_iter()
				.filter_map(|node| match node.location() {
					Some(location) => Some((location.clone(), node)),
					None => None,
				})
				.collect(),
		}
	}


	#[derive(Node)]
	struct MyComponent {
		value: usize,
	}
	fn my_component(props: MyComponent) -> RsxNode {
		rsx! { <div>the value is {props.value}<slot /></div> }
	}

	// test a roundtrip split/join,
	#[test]
	fn split_join() {
		let some_val = 3;

		let page = rsx! {
			<div key str="value" num=32 ident=some_val>
				hello world
			</div>
		};

		let html1 = page.clone().bpipe(RsxToHtmlString::default()).unwrap();
		let page_template = RsxTemplateNode::from_rsx_node(&page).unwrap();
		let map = test_template_map(vec![page_template]);
		let node2 = map.apply_template(page).unwrap();
		let html2 = node2.bpipe(RsxToHtmlString::default()).unwrap();
		expect(html1).to_be(html2);
	}
	#[test]
	fn rsx_template_match_simple() {
		let some_val = 3;
		let mut node1 = rsx! {
			<div ident=some_val>
				<div ident=some_val />
			</div>
		};
		let node2_template = rsx_template! {
			<div ident=some_val>
				<div ident=some_val />
			</div>
		};
		expect(&RsxTemplateNode::from_rsx_node(&node1).unwrap())
			.not()
			.to_be(&node2_template);

		node1.remove_location();
		expect(&RsxTemplateNode::from_rsx_node(&node1).unwrap())
			.to_be(&node2_template);
	}
	#[test]
	fn rsx_template_match_complex() {
		let some_val = 3;

		let mut node1 = rsx! {
			<div key str="value" num=32 ident=some_val onclick=|_| {}>
				<p>
					hello <MyComponent value=3 foo:bar bazz:boo="32">
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};
		let node2_template = rsx_template! {
			<div key str="value" num=32 ident=some_val onclick=|_| {}>
				<p>
					hello <MyComponent value=3 foo:bar bazz:boo="32">
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};
		expect(&RsxTemplateNode::from_rsx_node(&node1).unwrap())
			.not()
			.to_be(&node2_template);

		node1.remove_location();
		expect(&RsxTemplateNode::from_rsx_node(&node1).unwrap())
			.to_be(&node2_template);
	}

	// test a roundtrip split/join,
	#[test]
	fn nested_templates() {
		let page = rsx! {
			<div>
				<MyComponent value=3>
					<div>some child</div>
				</MyComponent>
			</div>
		};



		let html1 = page.clone().bpipe(RsxToHtmlString::default()).unwrap();
		let page_template = RsxTemplateNode::from_rsx_node(&page).unwrap();
		// these templates are usually generated by statically looking at a file,
		// here we create one from a default MyComponent, so the value: 4 will
		// be ignored
		let my_component_template =
			RsxTemplateNode::from_rsx_node(&MyComponent { value: 4 }.render())
				.unwrap();
		let map = test_template_map(vec![page_template, my_component_template]);
		let node2 = map.apply_template(page).unwrap();
		let html2 = node2.bpipe(RsxToHtmlString::default()).unwrap();
		expect(&html1).to_be("<div><div data-beet-rsx-idx=\"3\">the value is 3<div>some child</div></div></div>");
		expect(html1).to_be(html2);
	}

	#[test]
	fn ignores_exterior_roots() {
		let comp = rsx! { <div>foo</div> };
		let should_exist = comp.clone();
		let should_not_exist = comp.with_location(RsxMacroLocation::new(
			WorkspacePathBuf::new("../"),
			1,
			1,
		));

		let map = test_template_map(vec![]);

		expect(map.apply_template(should_exist)).to_be_err();
		expect(map.apply_template(should_not_exist)).to_be_ok();
	}
}
