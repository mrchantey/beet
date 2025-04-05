use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use sweet::prelude::*;
// use syn::Type;


#[derive(Debug, Default)]
pub struct FileGroupToFuncTokens {
	// pub frontmatter_ty: Type,
}



impl Pipeline<FileGroup, Result<Vec<FuncTokens>>> for FileGroupToFuncTokens {
	fn apply(self, group: FileGroup) -> Result<Vec<FuncTokens>> {
		let items = group
			.collect_files()?
			.into_iter()
			.enumerate()
			.map(|(index, file)| self.map_file(index, &group.src, file))
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.flatten()
			.collect::<Vec<_>>();

		Ok(items)
	}
}

impl FileGroupToFuncTokens {
	fn map_file(
		&self,
		index: usize,
		group_src: &CanonicalPathBuf,
		canonical_path: CanonicalPathBuf,
	) -> Result<Vec<FuncTokens>> {
		let file_str = ReadFile::to_string(&canonical_path)?;
		let local_path = PathExt::create_relative(&group_src, &canonical_path)?;
		match canonical_path.extension() {
			Some(ex) if ex == "rs" => FuncFileToFuncTokens::parse(
				index,
				&file_str,
				canonical_path,
				local_path,
			),
			// TODO html parsing
			// #[cfg(feature = "markdown")]
			// Some(ex) if ex == "md" => MarkdownToFuncTokens::parse(
			// 	&file_str,
			// 	canonical_path,
			// 	local_path,
			// )
			// .map(|func| vec![func]),
			_ => Ok(Vec::default()),
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_rsx::prelude::*;
	use quote::ToTokens;
	use sweet::prelude::*;

	#[test]
	#[ignore = "todo html parsing"]
	fn markdown() {
		let funcs = FileGroup::test_site_markdown()
			.xpipe(FileGroupToFuncTokens::default())
			.unwrap();
		expect(funcs.len()).to_be(1);
		let func_tokens = funcs
			.iter()
			.find(|f| f.local_path.ends_with("hello.md"))
			.unwrap();
		// expect(func_tokens.funcs.len()).to_be(1);
		expect(&func_tokens.local_path.to_string_lossy()).to_be("hello.md");
		expect(
			func_tokens.canonical_path.to_string_lossy().ends_with(
				"crates/beet_router/src/test_site/test_docs/hello.md",
			),
		)
		.to_be_true();
	}
	#[test]
	fn beet_site() {
		let docs = FileGroup::new_workspace_rel("crates/beet_site/src/docs")
			.unwrap()
			.xpipe(FileGroupToFuncTokens::default())
			.unwrap()
			.xpipe(MapFuncTokensRoute::default().base_route("/docs"))
			.xpipe(FuncTokensToCodegen::new(CodegenFile::new_workspace_rel(
				"crates/beet_site/src/codegen/docs.rs",
				"beet_site",
			)))
			.unwrap();
		println!(
			"{}",
			docs.1.build_output().unwrap().to_token_stream().to_string()
		);
	}
}
