use super::FileGroup;
use anyhow::Result;
use beet_rsx::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;
use sweet::prelude::*;
use syn::File;

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildFileComponents {
	pub file_group: FileGroup,
	/// The output codegen file location.
	pub output: WorkspacePathBuf,
}

impl Default for BuildFileComponents {
	fn default() -> Self {
		Self {
			file_group: "src/components".into(),
			output: "src/components/mod.rs".into(),
		}
	}
}

impl BuildFileComponents {
	pub fn new(
		file_group: impl Into<FileGroup>,
		output: impl Into<WorkspacePathBuf>,
	) -> Self {
		Self {
			file_group: file_group.into(),
			output: output.into(),
		}
	}

	/// A common configuration of [`BuildFileComponents`] is to collect all mockup files in a directory.
	pub fn mockups(src_dir: impl Into<WorkspacePathBuf>) -> Self {
		let src_dir = src_dir.into();
		let output = src_dir.join("codegen/mockups.rs");
		Self {
			file_group: FileGroup::new(src_dir)
				.with_filter(GlobFilter::default().with_include("*.mockup.rs")),
			output: output.into(),
		}
	}

	pub fn build_output(&self) -> Result<File> {
		let canonical_src = self.file_group.src.into_canonical()?;
		let canonical_out = self.output.into_canonical_unchecked()?;
		let output_dir = canonical_out.parent().ok_or_else(|| {
			anyhow::anyhow!("Output path must have a parent directory")
		})?;
		let files = self
			.file_group
			.collect_files()?
			.iter()
			.map(|file| {
				self.build_file_output(&canonical_src, &output_dir, &file)
			})
			.collect::<Result<Vec<_>>>()?;


		Ok(syn::parse_quote! {
			use beet::prelude::*;
			pub fn collect() -> Vec<FileComponent> {
				vec![#(#files),*]
			}
		})
	}

	pub fn build_file_output(
		&self,
		src: &CanonicalPathBuf,
		canonical_out_dir: &Path,
		file: &Path,
	) -> Result<TokenStream> {
		let canonical_file = CanonicalPathBuf::new(file)?;

		let output_relative =
			PathExt::create_relative(canonical_out_dir, &canonical_file)?;
		let output_relative_str = output_relative.to_string_lossy();
		let src_relative = PathExt::create_relative(src, &canonical_file)?;
		let src_relative_str = src_relative.to_string_lossy();

		Ok(quote! {{
		#[path=#output_relative_str]
			mod component;
			FileComponent::new(
				#src_relative_str,
				component::get
			)
		}})
	}

	pub fn build_and_write(&self) -> Result<()> {
		let output_tokens = self.build_output()?;
		let output_str = prettyplease::unparse(&output_tokens);

		let output_dir = self.output.into_canonical_unchecked()?;
		FsExt::write(&output_dir, &output_str)?;
		Ok(())
	}
}

impl BuildStep for BuildFileComponents {
	fn run(&self) -> Result<()> { self.build_and_write() }
}

pub struct FileComponent<T> {
	/// The path relative to the [`FileGroup::src`] it was collected from
	pub src_relative: PathBuf,
	pub func: T,
}
impl<T> FileComponent<T> {
	pub fn new(src_relative: impl AsRef<Path>, func: impl Into<T>) -> Self {
		Self {
			src_relative: src_relative.as_ref().into(),
			func: func.into(),
		}
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use quote::ToTokens;

	#[test]
	fn works() {
		let files =
			BuildFileComponents::mockups("crates/beet_router/src/test_site")
				.build_output()
				.unwrap()
				.to_token_stream()
				.to_string();
		println!("{}", files);

		// let paths = config.build_strings().unwrap();
		// expect(paths.len()).to_be(2);
		// println!("{:#?}", paths);
	}
}
