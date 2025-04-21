use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use sweet::prelude::*;
use syn::Ident;
// use syn::Type;


#[derive(Debug, Default)]
pub struct FileGroupToFuncTokens {
	// pub frontmatter_ty: Type,
}



impl Pipeline<FileGroup, Result<FuncTokensGroup>> for FileGroupToFuncTokens {
	fn apply(self, group: FileGroup) -> Result<FuncTokensGroup> {
		let items = group
			.collect_files()?
			.into_iter()
			.enumerate()
			.map(|(index, file)| self.map_file(index, &group.src, file))
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.flatten()
			.collect::<Vec<_>>();

		Ok(items.into())
	}
}

impl FileGroupToFuncTokens {
	fn map_file(
		&self,
		index: usize,
		group_src: &AbsPathBuf,
		canonical_path: AbsPathBuf,
	) -> Result<Vec<FuncTokens>> {
		let file_str = ReadFile::to_string(&canonical_path)?;
		let local_path = PathExt::create_relative(&group_src, &canonical_path)?;
		let mod_ident = Ident::new(
			&format!("file{}", index),
			proc_macro2::Span::call_site(),
		);

		match canonical_path.extension() {
			Some(ex) if ex == "rs" => FuncFileToFuncTokens::parse(
				mod_ident,
				&file_str,
				canonical_path,
				local_path,
			),
			#[cfg(feature = "markdown")]
			Some(ex) if ex == "md" || ex == "mdx" => MarkdownToFuncTokens::parse(
				mod_ident,
				&file_str,
				canonical_path,
				local_path,
			)
			.map(|func| vec![func]),
			_ => Ok(Vec::default()),
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_rsx::prelude::*;
	use sweet::prelude::*;

	#[test]
	#[ignore = "todo html parsing"]
	fn markdown() {
		let group = FileGroup::test_site_markdown()
			.xpipe(FileGroupToFuncTokens::default())
			.unwrap();
		expect(group.len()).to_be(1);
		let func_tokens = group
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
		let _docs = FileGroup::new(
			AbsPathBuf::new_workspace_rel("crates/beet_site/src/docs").unwrap(),
		)
		.xpipe(FileGroupToFuncTokens::default())
		.unwrap()
		.xpipe(MapFuncTokens::default().base_route("/docs"))
		.xpipe(FuncTokensToRsxRoutes::new(
			CodegenFile::new(
				AbsPathBuf::new_workspace_rel(
					"crates/beet_site/src/codegen/docs.rs",
				)
				.unwrap(),
			)
			.with_pkg_name("beet_site"),
		))
		.unwrap();
		// println!(
		// 	"{}",
		// 	docs.1.build_output().unwrap().to_token_stream().to_string()
		// );
	}
}
