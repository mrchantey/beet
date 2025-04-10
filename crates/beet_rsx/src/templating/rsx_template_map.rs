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
impl Pipeline<RsxNode, TemplateResult<RsxNode>> for &RsxTemplateMap {
	fn apply(self, mut node: RsxNode) -> TemplateResult<RsxNode> {
		let mut result = Ok(());
		// templates cannot track block initials and component nodes,
		// so we certainly must visit them
		// even though they can track element and slot children, we still need to visit
		// because the parent may not have a location/template, in most of these cases
		// the child will not have a location so itll be a noop.
		VisitRsxNodeMut::walk(&mut node, |node| {
			// this will atually mutate the node, effecting which children get visited next.
			if let Err(err) = self.apply_template(node) {
				result = Err(err);
			}
		});
		result.map(|_| node)
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

	fn apply_template(&self, node: &mut RsxNode) -> TemplateResult<()> {
		let Some(location) = node.location().cloned() else {
			// if the node doesnt have a location we dont even try to apply a template
			return Ok(());
		};
		// println!("applying template to node: {}", node.location_str());

		if let Some(template) = self.templates.get(&location) {
			// clone because multiple nodes may have the same location
			*node = (std::mem::take(node), template.clone())
				.xpipe(ApplyTemplateToNode)
				.map_err(|err| err.with_location(location.clone()))?;
			Ok(())
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
			// if the node location is outside the templates root directory,
			// it wouldn't be expected to have a template.
			Ok(())
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

	#[derive(Default, Node)]
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
		expect(&node1.xref().xpipe(NodeToTemplate).unwrap())
			.not()
			.to_be(&node2_template);

		node1.remove_location();
		expect(&node1.xref().xpipe(NodeToTemplate).unwrap())
			.to_be(&node2_template);
	}
	#[test]
	fn rsx_template_match_complex() {
		let some_val = 3;

		let mut node1 = rsx! {
			<div key str="value" num=32 ident=some_val onclick=|_| {}>
				<p>
					hello <MyComponent  scope:cascade value=3>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};
		let node2_template = rsx_template! {
			<div key str="value" num=32 ident=some_val onclick=|_| {}>
				<p>
					hello <MyComponent scope:cascade value=3>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};
		expect(&node1.xref().xpipe(NodeToTemplate).unwrap())
			.not()
			.to_be(&node2_template);

		node1.remove_location();
		expect(&node1.xref().xpipe(NodeToTemplate).unwrap())
			.to_be(&node2_template);
	}
	/// TODO this doesnt test trackers generated via syn::parse which generates whitespace etc differently
	#[test]
	fn trackers_match() {
		let node1 =
			rsx! {<MyComponent scope:cascade value=3>Hello</MyComponent>};
		let node2_template = rsx_template! {<MyComponent scope:cascade value=3>Hello</MyComponent>};
		let RsxNode::Component(RsxComponent {
			tracker: tracker1, ..
		}) = node1
		else {
			panic!();
		};
		let RsxTemplateNode::Component {
			tracker: tracker2, ..
		} = node2_template
		else {
			panic!();
		};
		expect(tracker1).to_be(tracker2);
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
	fn ignores_exterior_roots() {
		let map = test_template_map(vec![]);
		let comp = rsx! { <div>foo</div> };

		// interior root, not found
		expect(comp.clone().xpipe(&map)).to_be_err();
		// exterior root, ok
		expect(
			comp.with_location(RsxMacroLocation::new(
				WorkspacePathBuf::new("../"),
				1,
				1,
			))
			.xpipe(&map),
		)
		.to_be_ok();
	}
}
