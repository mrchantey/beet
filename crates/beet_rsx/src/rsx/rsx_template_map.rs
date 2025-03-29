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
/// When joining an [RsxTemplateRoot] with an [RustyPartMap],
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
	pub templates: HashMap<RsxMacroLocation, RsxTemplateRoot>,
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

	// TODO pipeline
	/// ## Errors
	/// If the root is inside the templates root directory and a template was not found.
	pub fn apply_template(&self, root: RsxRoot) -> TemplateResult<RsxRoot> {
		if let Some(template_root) = self.templates.get(&root.location) {
			let node = self
				.apply_template_for_node(
					template_root.clone(),
					&mut RustyPartMap::collect(root.node),
				)
				.map_err(|err| err.with_location(root.location.clone()))?;
			Ok(node)
		} else if root.location.file.starts_with(&self.root) {
			Err(TemplateError::NoTemplate {
				received: self
					.templates
					.values()
					.map(|x| x.location.clone())
					.collect(),
				expected: root.location.clone(),
			}
			.with_location(root.location.clone()))
		} else {
			println!(
				"rsx node is outside templates dir so no template will be applied:\n{:?}",
				root.location
			);
			Ok(root)
		}
	}

	// fn template_should_exist(&self, location: &RsxMacroLocation) -> bool {
	// 	location.file.starts
	// }


	/// Create an [`RsxRoot`] from a template and hydrated nodes.
	fn apply_template_for_node(
		&self,
		root: RsxTemplateRoot,
		rusty_map: &mut RustyPartMap,
	) -> TemplateResult<RsxRoot> {
		// i dont like passing self like this, kind of hides recursion
		let node = root.node.into_rsx_node(self, rusty_map)?;
		Ok(RsxRoot {
			node,
			location: root.location,
		})
	}
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;


	/// used for testing, load directly from a collection of template roots.
	#[cfg(test)]
	pub fn test_template_map(
		templates: Vec<RsxTemplateRoot>,
	) -> RsxTemplateMap {
		RsxTemplateMap {
			root: WorkspacePathBuf::new(file!()),
			templates: templates
				.into_iter()
				.map(|root| (root.location.clone(), root))
				.collect(),
		}
	}


	#[derive(Node)]
	struct MyComponent {
		value: usize,
	}
	fn my_component(props: MyComponent) -> RsxRoot {
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

		let html1 = page.clone().pipe(RsxToHtmlString::default()).unwrap();
		let page_template = RsxTemplateRoot::from_rsx(&page).unwrap();
		let map = test_template_map(vec![page_template]);
		let node2 = map.apply_template(page).unwrap();
		let html2 = node2.pipe(RsxToHtmlString::default()).unwrap();
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
		};
		let node1_template = RsxTemplateRoot::from_rsx(&node1).unwrap();
		expect(&node1_template).not().to_be(&node2_template);
		expect(&node1_template.node).to_be(&node2_template.node);
	}
	#[test]
	fn rsx_template_match_complex() {
		let some_val = 3;

		let node1 = rsx! {
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
		let node1_template = RsxTemplateRoot::from_rsx(&node1).unwrap();
		expect(&node1_template).not().to_be(&node2_template);
		expect(&node1_template.node).to_be(&node2_template.node);
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



		let html1 = page.clone().pipe(RsxToHtmlString::default()).unwrap();
		let page_template = RsxTemplateRoot::from_rsx(&page).unwrap();
		// these templates are usually generated by statically looking at a file,
		// here we create one from a default MyComponent, so the value: 4 will
		// be ignored
		let my_component_template =
			RsxTemplateRoot::from_rsx(&MyComponent { value: 4 }.render())
				.unwrap();
		let map = test_template_map(vec![page_template, my_component_template]);
		let node2 = map.apply_template(page).unwrap();
		let html2 = node2.pipe(RsxToHtmlString::default()).unwrap();
		expect(&html1).to_be("<div><div data-beet-rsx-idx=\"3\">the value is 3<div>some child</div></div></div>");
		expect(html1).to_be(html2);
	}

	#[test]
	fn ignores_exterior_roots() {
		let comp = rsx! { <div>foo</div> };
		let should_exist = comp.clone();
		let mut should_not_exist = comp;
		should_not_exist.location =
			RsxMacroLocation::new(WorkspacePathBuf::new("../"), 1, 1);

		let map = test_template_map(vec![]);

		expect(map.apply_template(should_exist)).to_be_err();
		expect(map.apply_template(should_not_exist)).to_be_ok();
	}
}
