use std::path::Path;

use crate::prelude::*;
use anyhow::Result;
use sweet::prelude::*;




/// Resolve the src attribute to a file if it does not start with any [IGNORED_PREFIX]:
pub struct FsSrcPipeline {}

const IGNORED_PREFIX: [&str; 3] = ["/", "http://", "https://"];

impl Default for FsSrcPipeline {
	fn default() -> Self { Self {} }
}

impl RsxPipeline<RsxRoot> for FsSrcPipeline {
	fn apply(self, mut root: RsxRoot) -> Result<RsxRoot> {
		//1. apply to root
		self.apply_root(&mut root)?;

		let mut result = Ok(());

		//2. apply to the root of each component
		VisitRsxComponentMut::walk(&mut root, |component| {
			if let Err(err) = self.apply_root(&mut component.root) {
				result = Err(err);
			}
		});

		result.map(|_| root)
	}
}

impl FsSrcPipeline {
	/// apply to a root without recursing into components
	fn apply_root(&self, root: &mut RsxRoot) -> Result<()> {
		let mut result = Ok(());

		VisitRsxElementMut::walk_with_opts(
			&mut root.node,
			VisitRsxOptions::ignore_component_node(),
			|el| {
				if let Some(src) = el.get_key_value_attr("src") {
					if IGNORED_PREFIX
						.iter()
						.any(|prefix| src.starts_with(prefix))
					{
						return;
					}

					let file = Path::new(&root.location.file);
					let workspace_path =
						file.parent().unwrap_or(&Path::new("")).join(src);
					// we use the workspace root because location.file! is relative to the workspace
					let path = FsExt::workspace_root().join(workspace_path);

					if !el.children.is_empty() {
						result = Err(anyhow::anyhow!(
							"elements with an fs src attribute cannot have children"
						));
					}
					el.remove_matching_key("src");

					match ReadFile::to_string(&path) {
						Ok(value) => {
							el.self_closing = false;
							el.children = Box::new(RsxNode::Text {
								idx: rsx_idx_invalid(),
								value,
							})
						}
						Err(err) => result = Err(err.into()),
					}
				}
			},
		);
		result
	}
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[derive(Node)]
	struct Foo;

	fn foo(_: Foo) -> RsxRoot {
		rsx! {
			<div>
				<slot />
			</div>
		}
	}


	#[test]
	fn works() {
		// relative ignored
		expect(
			rsx! { <script src="/missing" /> }.pipe(FsSrcPipeline::default()),
		)
		.to_be_ok();
		// missing errors
		expect(
			rsx! { <script src="missing" /> }.pipe(FsSrcPipeline::default()),
		)
		.to_be_err();
		// slot children errors
		expect(
			rsx! {
				<Foo>
					<script src="missing" />
				</Foo>
			}
			.pipe(FsSrcPipeline::default()),
		)
		.to_be_err();

		let root = rsx! { <script src="test.js" /> }
			.pipe(FsSrcPipeline::default())
			.unwrap();

		let RsxNode::Element(el) = &root.node else {
			panic!()
		};
		let RsxNode::Text { value, .. } = el.children.as_ref() else {
			panic!()
		};
		expect(value).to_be(include_str!("test.js"));
	}
}
