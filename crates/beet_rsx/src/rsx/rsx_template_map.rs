use crate::prelude::*;
use anyhow::Result;



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
#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RsxTemplateMap(pub HashMap<RsxLocation, RsxTemplateRoot>);


impl RsxTemplateMap {
	/// Load the template map serialized by [beet_rsx_parser::RstmlToRsxTemplate]
	#[cfg(all(feature = "serde", not(target_arch = "wasm32")))]
	pub fn load(src: &std::path::Path) -> Result<Self> {
		use sweet::prelude::ReadFile;
		{
			let tokens = ReadFile::to_string(src)?;
			let this: Self = ron::de::from_str(&tokens.to_string())?;
			Result::Ok(this)
		}
	}

	/// used for testing, load directly from a collection of template roots.
	pub fn from_template_roots(roots: Vec<RsxTemplateRoot>) -> Self {
		Self(
			roots
				.into_iter()
				.map(|root| (root.location.clone(), root))
				.collect(),
		)
	}

	// should live elsewhere, maybe RustyPart
	pub fn apply_template(&self, root: RsxRoot) -> TemplateResult<RsxRoot> {
		let mut rusty_map = RustyPartMap::collect(root.node)?;
		let location = root.location;
		// i think here we need to pass the whole map for component template reloading
		let template_root = self
			.get(&location)
			.ok_or_else(|| TemplateError::NoTemplate(location.clone()))?
			.clone();
		let node =
			self.apply_template_for_node(template_root, &mut rusty_map)?;
		Ok(node)
	}

	/// Create an [`RsxRoot`] from a template and hydrated nodes.
	/// 		todo!("this is wrong, we need template map for each component?;
	fn apply_template_for_node(
		&self,
		root: RsxTemplateRoot,
		rusty_map: &mut RustyPartMap,
	) -> TemplateResult<RsxRoot> {
		let node = root.node.into_rsx_node(self, rusty_map)?;
		Ok(RsxRoot {
			node,
			location: root.location,
		})
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	struct MyComponent {
		value: usize,
	}
	impl Component for MyComponent {
		fn render(self) -> RsxRoot {
			rsx! { <div>the value is {self.value}<slot /></div> }
		}
	}

	// test a roundtrip split/join,
	#[test]
	fn split_join() {
		let some_val = 3;

		let page = || {
			rsx! {
				<div key str="value" num=32 ident=some_val>
					<p>
						hello <MyComponent value=3>
							<div>some child</div>
						</MyComponent>
					</p>
				</div>
			}
		};


		let html1 = page().render_body();
		let page_template = RsxTemplateRoot::from_rsx(&page()).unwrap();
		let my_component_template =
			RsxTemplateRoot::from_rsx(&MyComponent { value: 4 }.render())
				.unwrap();
		// println!("{:#?}", page_template);
		let map = RsxTemplateMap::from_template_roots(vec![
			page_template,
			my_component_template,
		]);
		let node2 = map.apply_template(page()).unwrap();
		let html2 = node2.render_body();
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
					hello <MyComponent value=3>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};
		let node2_template = rsx_template! {
			<div key str="value" num=32 ident=some_val onclick=|_| {}>
				<p>
					hello <MyComponent value=3>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};
		let node1_template = RsxTemplateRoot::from_rsx(&node1).unwrap();
		expect(&node1_template).not().to_be(&node2_template);
		expect(&node1_template.node).to_be(&node2_template.node);
	}
}
