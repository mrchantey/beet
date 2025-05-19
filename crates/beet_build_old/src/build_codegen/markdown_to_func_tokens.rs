use crate::prelude::*;
use anyhow::Result;
use beet_router::prelude::*;
use beet_rsx::prelude::*;
use beet_rsx_parser::prelude::*;
use std::path::PathBuf;
use sweet::prelude::*;
use syn::Ident;
use syn::ItemFn;

pub struct MarkdownToFuncTokens;

// impl MarkdownToFuncTokens

impl MarkdownToFuncTokens {
	pub fn parse(
		mod_ident: Ident,
		markdown: &str,
		abs_path: AbsPathBuf,
		local_path: PathBuf,
	) -> Result<FuncTokens> {
		let workspace_path = WorkspacePathBuf::new_from_cwd_rel(&abs_path)?;
		let frontmatter =
			ParseMarkdown::markdown_to_frontmatter_tokens(markdown)?;
		let rsx_str = ParseMarkdown::markdown_to_rsx_str(markdown);
		let rust_tokens = rsx_str
			.xref()
			.xpipe(StringToWebTokens::new(workspace_path))
			.map_err(|e| {
				anyhow::anyhow!(
					"Failed to parse Markdown HTML\nPath: {}\nInput: {}\nError: {}",
					abs_path.display(),
					rsx_str,
					e.to_string()
				)
			})?
			.xpipe(WebTokensToRust::default());

		let item_fn: ItemFn = syn::parse_quote! {
			pub fn get() -> WebNode
				#rust_tokens

		};

		Ok(FuncTokens {
			mod_ident: mod_ident.clone(),
			mod_import: ModImport::Inline,
			frontmatter,
			item_fn,
			route_info: RouteInfo {
				path: RoutePath::from_file_path(&local_path)?,
				method: HttpMethod::Get,
			},
			local_path,
			abs_path,
		})
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use quote::ToTokens;
	use serde::Deserialize;
	use serde::Serialize;
	use sweet::prelude::*;
	use syn::ItemFn;

	const MARKDOWN: &str = r#"
+++
val_bool		=	true
val_int			= 83
val_float		= 3.14
val_string	=	"bar=bazz"
[val_enum]
Bar 				= 42
[val_nested]
val_string	= "foo"
+++
# hello world"#;


	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	enum MyEnum {
		Foo,
		Bar(u32),
	}
	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	struct Frontmatter {
		val_bool: bool,
		val_int: u32,
		val_float: Option<f32>,
		val_string: String,
		val_enum: MyEnum,
		val_nested: Nested,
	}

	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	struct Nested {
		val_string: String,
	}

	#[test]
	fn parse() {
		let func_tokens = MarkdownToFuncTokens::parse(
			syn::parse_quote!(foo),
			MARKDOWN,
			AbsPathBuf::new_workspace_rel_unchecked(file!()),
			"bar".into(),
		)
		.unwrap();

		let expected: ItemFn = syn::parse_quote! {
		pub fn get() -> WebNode {
			use beet::prelude::*;
			#[allow(unused_braces)]
			RsxElement {
				tag: "h1".to_string(),
				attributes: vec![],
				children: Box::new(
					RsxText {
						value: "hello world".to_string(),
						meta: NodeMeta::new(
							FileSpan::new("crates/beet_build/src/build_codegen/markdown_to_func_tokens.rs",
								LineCol::new(1, 0),
								LineCol::new(1, 0))
								, vec![]
							),
					}.into_node()
				),
				self_closing: false,
				meta: NodeMeta::new(
					FileSpan::new("crates/beet_build/src/build_codegen/markdown_to_func_tokens.rs",
						LineCol::new(1, 0),
						LineCol::new(1, 0))
						, vec![TemplateDirective::NodeTemplate]
					),
		}.into_node()
		}
		};

		expect(func_tokens.item_fn.to_token_stream().to_string())
			.to_be(expected.to_token_stream().to_string());
	}
}
