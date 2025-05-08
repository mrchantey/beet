use crate::prelude::*;
use anyhow::Result;
use beet_common::prelude::*;
use std::path::Path;
use sweet::prelude::*;

/// Resolve the src attribute to a file if it starts with a dot
pub struct ApplyFsSrc;

impl Default for ApplyFsSrc {
	fn default() -> Self { Self {} }
}

impl Pipeline<WebNode, Result<WebNode>> for ApplyFsSrc {
	fn apply(self, mut root: WebNode) -> Result<WebNode> {
		//1. apply to root
		self.apply_to_node(&mut root)?;

		let mut result = Ok(());

		//2. apply to the root of each component
		VisitRsxComponentMut::walk(&mut root, |component| {
			if let Err(err) = self.apply_to_node(&mut component.node) {
				result = Err(err);
			}
		});

		result.map(|_| root)
	}
}

impl ApplyFsSrc {
	/// apply to a root without recursing into components
	fn apply_to_node(&self, node: &mut WebNode) -> Result<()> {
		let mut result = Ok(());
		let location = node.location().cloned();
		VisitRsxElementMut::walk_with_opts(
			node,
			VisitRsxOptions::ignore_component_node(),
			|el| {
				if let Some(src) = el.src_directive() {
					let Some(location) = &location else {
						result = Err(anyhow::anyhow!(
							"elements with an fs src attribute must have a RootNode::location. This is set by default in rsx! macros"
						));
						return;
					};

					let workspace_path = location
						.file()
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

	fn foo(_: Foo) -> WebNode {
		rsx! {
			<div>
				<slot />
			</div>
		}
	}


	#[test]
	fn works() {
		// relative ignored
		expect(rsx! { <script src="/missing" /> }.xpipe(ApplyFsSrc::default()))
			.to_be_ok();
		// missing errors
		expect(
			rsx! { <script src="./missing" /> }.xpipe(ApplyFsSrc::default()),
		)
		.to_be_err();
		// slot children errors
		expect(
			rsx! {
				<Foo>
					<script src="./missing" />
				</Foo>
			}
			.xpipe(ApplyFsSrc::default()),
		)
		.to_be_err();

		let node = rsx! { <script src="./test-fs-src.js" /> }
			.xpipe(ApplyFsSrc::default())
			.unwrap();

		let WebNode::Element(el) = &node else {
			panic!()
		};
		let WebNode::Text(text) = el.children.as_ref() else {
			panic!()
		};
		expect(&text.value).to_be(include_str!("./test-fs-src.js"));
	}
}
