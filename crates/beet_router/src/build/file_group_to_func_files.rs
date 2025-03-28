use crate::prelude::*;
use anyhow::Result;
use beet_rsx::rsx::RsxPipeline;
use std::path::PathBuf;
use sweet::prelude::CanonicalPathBuf;
use sweet::prelude::ReadFile;
use sweet::prelude::*;
use syn::Ident;
use syn::Visibility;

/// For a given file group, collect all public functions.
#[derive(Debug, Default, Clone)]
pub struct FileGroupToFuncFiles;

impl RsxPipeline<FileGroup, Result<Vec<FuncFile>>> for FileGroupToFuncFiles {
	fn apply(self, group: FileGroup) -> Result<Vec<FuncFile>> {
		group
			.collect_files()?
			.into_iter()
			.enumerate()
			.map(|(i, p)| self.build_func_file(i, &group.src, p))
			.collect::<Result<Vec<_>>>()
	}
}


impl FileGroupToFuncFiles {
	fn build_func_file(
		&self,
		index: usize,
		group_src: &CanonicalPathBuf,
		file: PathBuf,
	) -> Result<FuncFile> {
		let file_str = ReadFile::to_string(&file)?;
		let funcs = syn::parse_file(&file_str)?
			.items
			.into_iter()
			.filter_map(|item| {
				if let syn::Item::Fn(f) = item {
					match &f.vis {
						Visibility::Public(_) => {
							return Some(f.sig);
						}
						_ => {}
					}
				}
				None
			})
			.collect::<Vec<_>>();

		let canonical_path = CanonicalPathBuf::new(file)?;
		let local_path = PathExt::create_relative(&group_src, &canonical_path)?;
		let ident = Ident::new(
			&format!("file{}", index),
			proc_macro2::Span::call_site(),
		);

		Ok(FuncFile {
			ident,
			canonical_path,
			local_path,
			funcs,
		})
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_rsx::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let funcs = FileGroup::test_site_routes()
			.pipe(FileGroupToFuncFiles::default())
			.unwrap();
		expect(funcs.len()).to_be(3);
		let file = funcs
			.iter()
			.find(|f| f.local_path.ends_with("docs/index.rs"))
			.unwrap();
		expect(file.funcs.len()).to_be(1);
		expect(&file.local_path.to_string_lossy()).to_be("docs/index.rs");
		expect(file.canonical_path.to_string_lossy().ends_with(
			"crates/beet_router/src/test_site/routes/docs/index.rs",
		))
		.to_be_true();
	}
}
