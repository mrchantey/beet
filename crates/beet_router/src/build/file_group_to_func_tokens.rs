use crate::prelude::*;
use anyhow::Result;
use beet_rsx::rsx::RsxPipeline;
use beet_rsx::rsx::RsxPipelineTarget;
// use syn::Type;


#[derive(Debug, Default)]
pub struct FileGroupToFuncTokens {
	// pub frontmatter_ty: Type,
}



impl RsxPipeline<FileGroup, Result<Vec<FuncTokens>>> for FileGroupToFuncTokens {
	fn apply(self, group: FileGroup) -> Result<Vec<FuncTokens>> {
		group
			.collect_files()?
			.into_iter()
			.filter_map(|file| match file.extension() {
				#[cfg(feature = "markdown")]
				Some(ext) if ext == "md" => {
					Some(file.bpipe(MarkdownToFuncTokens::new(&group.src)))
				}
				_ => return None,
			})
			.collect::<Result<Vec<_>>>()
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_rsx::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let funcs = FileGroup::test_site_markdown()
			.bpipe(FileGroupToFuncTokens::default())
			.unwrap();
		expect(funcs.len()).to_be(1);
		let func_tokens = funcs
			.iter()
			.find(|f| f.local_path.ends_with("hello.md"))
			.unwrap();
		// expect(func_tokens.funcs.len()).to_be(1);
		expect(&func_tokens.local_path.to_string_lossy())
			.to_be("hello.md");
		expect(func_tokens.canonical_path.to_string_lossy().ends_with(
			"crates/beet_router/src/test_site/test_docs/hello.md",
		))
		.to_be_true();
	}
}
