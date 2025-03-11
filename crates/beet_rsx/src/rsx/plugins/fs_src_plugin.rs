use std::path::Path;

use crate::prelude::*;
use anyhow::Result;
use sweet::prelude::*;




/// Resolve the src attribute to a file if it does not start with any [IGNORED_PREFIX]:
pub struct FsSrcPlugin {}

const IGNORED_PREFIX: [&str; 3] = ["/", "http://", "https://"];

impl Default for FsSrcPlugin {
	fn default() -> Self { Self {} }
}

impl RsxPlugin for FsSrcPlugin {
	fn apply(self, root: &mut RsxRoot) -> Result<()> {
		//1. apply to root
		self.apply_root(root)?;

		let mut result = Ok(());

		//2. apply to the root of each component
		VisitRsxComponentMut::walk(root, |component| {
			if let Err(err) = self.apply_root(&mut component.root) {
				result = Err(err);
			}
		});

		result
	}
}

impl FsSrcPlugin {
	/// apply to a root without recursing into components
	fn apply_root(&self, root: &mut RsxRoot) -> Result<()> {
		let mut result = Ok(());

		VisitRsxElementMut::walk_with_opts(
			&mut root.node,
			VisitRsxOptions::ignore_component(),
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

	#[test]
	fn works() {
		// relative ignored
		expect(
			FsSrcPlugin::default()
				.apply(&mut rsx! { <script src="/missing" /> }),
		)
		.to_be_ok();
		// missing errors
		expect(
			FsSrcPlugin::default()
				.apply(&mut rsx! { <script src="missing" /> }),
		)
		.to_be_err();

		let mut root = rsx! { <script src="test.js" /> };
		expect(FsSrcPlugin::default().apply(&mut root)).to_be_ok();

		let RsxNode::Element(el) = &root.node else {
			panic!()
		};
		let RsxNode::Text { value, .. } = el.children.as_ref() else {
			panic!()
		};
		expect(value).to_be(include_str!("test.js"));
	}
}
