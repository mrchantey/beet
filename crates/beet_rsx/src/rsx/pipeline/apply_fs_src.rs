use std::path::Path;

use crate::prelude::*;
use anyhow::Result;
use sweet::prelude::*;




/// Resolve the src attribute to a file if it does not start with any [IGNORED_PREFIX]:
pub struct ApplyFsSrc;

const IGNORED_PREFIX: [&str; 3] = ["/", "http://", "https://"];

impl Default for ApplyFsSrc {
	fn default() -> Self { Self {} }
}

impl RsxPipeline<RsxNode, Result<RsxNode>> for ApplyFsSrc {
	fn apply(self, mut root: RsxNode) -> Result<RsxNode> {
		//1. apply to root
		self.apply_root(&mut root)?;

		let mut result = Ok(());

		//2. apply to the root of each component
		VisitRsxComponentMut::walk(&mut root, |component| {
			if let Err(err) = self.apply_root(&mut component.node) {
				result = Err(err);
			}
		});

		result.map(|_| root)
	}
}

impl ApplyFsSrc {
	/// apply to a root without recursing into components
	fn apply_root(&self, node: &mut RsxNode) -> Result<()> {
		let mut result = Ok(());
		let location = node.location().cloned();

		VisitRsxElementMut::walk_with_opts(
			node,
			VisitRsxOptions::ignore_component_node(),
			|el| {
				if let Some(src) = el.get_key_value_attr("src") {
					if IGNORED_PREFIX
						.iter()
						.any(|prefix| src.starts_with(prefix))
					{
						return;
					}

					let Some(location) = &location else {
						result = Err(anyhow::anyhow!(
							"elements with an fs src attribute must have a RootNode::location. This is set by default in rsx! macros"
						));
						return;
					};

					let workspace_path = location
						.file
						.parent()
						.unwrap_or(&Path::new(""))
						.join(src);
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
							el.children = Box::new(value.into_node());
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

	fn foo(_: Foo) -> RsxNode {
		rsx! {
			<div>
				<slot />
			</div>
		}
	}


	#[test]
	fn works() {
		// relative ignored
		expect(rsx! { <script src="/missing" /> }.bpipe(ApplyFsSrc::default()))
			.to_be_ok();
		// missing errors
		expect(rsx! { <script src="missing" /> }.bpipe(ApplyFsSrc::default()))
			.to_be_err();
		// slot children errors
		expect(
			rsx! {
				<Foo>
					<script src="missing" />
				</Foo>
			}
			.bpipe(ApplyFsSrc::default()),
		)
		.to_be_err();

		let node = rsx! { <script src="test-fs-src.js" /> }
			.bpipe(ApplyFsSrc::default())
			.unwrap();

		let RsxNode::Element(el) = &node else {
			panic!()
		};
		let RsxNode::Text(text) = el.children.as_ref() else {
			panic!()
		};
		expect(&text.value).to_be(include_str!("test-fs-src.js"));
	}
}
