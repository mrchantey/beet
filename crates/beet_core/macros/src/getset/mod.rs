//! Shared types and utilities for the `Get`, `GetMut`, `Set`, and `SetWith` derive macros.
extern crate alloc;

pub mod get;
pub mod get_mut;
pub mod set;
pub mod set_with;

use alloc::string::ToString;
use alloc::vec::Vec;
use beet_core_shared::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::Type;

/// Visibility of generated methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vis {
	Private,
	PubCrate,
	PubSuper,
	Pub,
}

impl Default for Vis {
	fn default() -> Self { Self::Pub }
}

impl Vis {
	/// Convert to a token stream for code generation.
	pub fn to_tokens(&self) -> TokenStream {
		match self {
			Vis::Private => quote! {},
			Vis::PubCrate => quote! { pub(crate) },
			Vis::PubSuper => quote! { pub(super) },
			Vis::Pub => quote! { pub },
		}
	}
}

/// Return type strategy for `Get` derive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetReturnType {
	/// Return `&T`.
	Ref,
	/// Return `T` via `.clone()`.
	Clone,
	/// Return `T` via copy.
	Copy,
}

impl Default for GetReturnType {
	fn default() -> Self { Self::Ref }
}

/// Kind of trait object wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraitWrapperKind {
	Box,
	Arc,
}

/// Resolved config for a single field.
pub struct FieldConfig {
	/// Visibility of the generated method.
	pub vis: Vis,
	/// Return type strategy, only used by `Get`.
	pub return_type: GetReturnType,
	/// Unwrap `Option<T>` in setters, only used by `Set`/`SetWith`.
	pub unwrap_option: bool,
	/// Unwrap trait object wrappers, used by `Get`/`Set`/`SetWith`.
	pub unwrap_trait: bool,
	/// Skip generation for this field.
	pub skip: bool,
	/// Accept `impl Into<T>` in the setter. Auto-enabled for String / Cow types.
	pub use_into: bool,
	/// Suppress the auto-into behaviour for String / Cow fields.
	pub not_into: bool,
}


/// Defaults parsed from struct-level attributes.
pub struct StructConfig {
	/// Default visibility for generated methods.
	pub vis: Vis,
	/// Default return type strategy.
	pub return_type: GetReturnType,
	/// Default unwrap option behavior.
	pub unwrap_option: bool,
	/// Default unwrap trait behavior.
	pub unwrap_trait: bool,
	/// Ignore this derive entirely, generating no methods.
	pub ignore: bool,
}

impl Default for StructConfig {
	fn default() -> Self {
		Self {
			vis: Vis::Pub,
			return_type: GetReturnType::Ref,
			unwrap_option: false,
			unwrap_trait: false,
			ignore: false,
		}
	}
}

/// Parse a visibility expression from an attribute value.
///
/// Note: `Pub` is the default and cannot be specified as an attribute value
/// because `pub` is a Rust keyword that cannot appear as an expression.
pub fn parse_vis(expr: &syn::Expr) -> syn::Result<Vis> {
	let syn::Expr::Path(path) = expr else {
		synbail!(
			expr,
			"Expected a visibility identifier: private, pub_crate, pub_super, pub"
		);
	};
	let Some(ident) = path.path.get_ident() else {
		synbail!(
			expr,
			"Expected a single identifier: private, pub_crate, pub_super, pub"
		);
	};
	match ident.to_string().as_str() {
		"private" => Ok(Vis::Private),
		"pub_crate" => Ok(Vis::PubCrate),
		"pub_super" => Ok(Vis::PubSuper),
		other => synbail!(
			ident,
			"Unknown visibility `{}`, expected: private, pub_crate, pub_super",
			other
		),
	}
}

const STRUCT_ALLOWED_KEYS: &[&str] = &[
	"vis",
	"clone",
	"copy",
	"unwrap_option",
	"unwrap_trait",
	"ignore",
];
const FIELD_ALLOWED_KEYS: &[&str] = &[
	"vis",
	"clone",
	"copy",
	"skip",
	"option",
	"unwrap_option",
	"unwrap_trait",
	"into",
	"not_into",
];

/// Extract the token content from an attribute, returning `None` for bare paths.
fn attr_tokens(
	attr: &syn::Attribute,
	attr_name: &str,
) -> syn::Result<Option<TokenStream>> {
	match &attr.meta {
		syn::Meta::List(list) => Ok(Some(list.tokens.clone())),
		syn::Meta::Path(_) => Ok(None),
		other => {
			synbail!(other, "Expected attribute list, ie #[{}(...)]", attr_name)
		}
	}
}

/// Apply common config keys from an [`AttributeMap`] to vis, return_type,
/// unwrap_option, unwrap_trait, use_into, and not_into.
fn apply_common_keys(
	map: &AttributeMap,
	vis: &mut Vis,
	return_type: &mut GetReturnType,
	unwrap_option: &mut bool,
	unwrap_trait: &mut bool,
	use_into: &mut bool,
	not_into: &mut bool,
) -> syn::Result<()> {
	if let Some(expr) = map.get("vis") {
		*vis = parse_vis(expr)?;
	}
	if map.contains_key("clone") {
		*return_type = GetReturnType::Clone;
	}
	if map.contains_key("copy") {
		*return_type = GetReturnType::Copy;
	}
	if map.contains_key("option") || map.contains_key("unwrap_option") {
		*unwrap_option = true;
	}
	if map.contains_key("unwrap_trait") {
		*unwrap_trait = true;
	}
	if map.contains_key("into") {
		*use_into = true;
	}
	if map.contains_key("not_into") {
		*not_into = true;
	}
	Ok(())
}

/// Parse struct-level attributes into a [`StructConfig`].
pub fn parse_struct_config(
	attrs: &[syn::Attribute],
	attr_name: &str,
) -> syn::Result<StructConfig> {
	let mut config = StructConfig::default();

	let Some(attr) = attrs.iter().find(|attr| attr.path().is_ident(attr_name))
	else {
		return Ok(config);
	};

	let Some(tokens) = attr_tokens(attr, attr_name)? else {
		return Ok(config);
	};

	let map = AttributeMap::parse(tokens)?;
	map.assert_types(&[], STRUCT_ALLOWED_KEYS)?;

	if map.contains_key("ignore") {
		config.ignore = true;
		return Ok(config);
	}

	let mut _use_into = false;
	let mut _not_into = false;
	apply_common_keys(
		&map,
		&mut config.vis,
		&mut config.return_type,
		&mut config.unwrap_option,
		&mut config.unwrap_trait,
		&mut _use_into,
		&mut _not_into,
	)?;

	Ok(config)
}

/// Parse field-level attributes into a [`FieldConfig`], inheriting from
/// struct defaults.
pub fn parse_field_config(
	attrs: &[syn::Attribute],
	attr_name: &str,
	defaults: &StructConfig,
) -> syn::Result<FieldConfig> {
	let mut config = FieldConfig {
		vis: defaults.vis,
		return_type: defaults.return_type,
		unwrap_option: defaults.unwrap_option,
		unwrap_trait: defaults.unwrap_trait,
		skip: false,
		use_into: false,
		not_into: false,
	};

	let Some(attr) = attrs.iter().find(|attr| attr.path().is_ident(attr_name))
	else {
		return Ok(config);
	};

	let Some(tokens) = attr_tokens(attr, attr_name)? else {
		return Ok(config);
	};

	let map = AttributeMap::parse(tokens)?;
	map.assert_types(&[], FIELD_ALLOWED_KEYS)?;

	if map.contains_key("skip") {
		config.skip = true;
		return Ok(config);
	}

	apply_common_keys(
		&map,
		&mut config.vis,
		&mut config.return_type,
		&mut config.unwrap_option,
		&mut config.unwrap_trait,
		&mut config.use_into,
		&mut config.not_into,
	)?;

	Ok(config)
}

/// If type is `Option<T>`, return the inner `T`.
pub fn option_inner_type(ty: &Type) -> Option<&Type> {
	let Type::Path(path) = ty else { return None };
	let segment = path.path.segments.last()?;
	if segment.ident != "Option" {
		return None;
	}
	let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
		return None;
	};
	match args.args.first()? {
		syn::GenericArgument::Type(inner) => Some(inner),
		_ => None,
	}
}

/// Detect `Box<dyn Trait>` or `Arc<dyn Trait>`, returning the wrapper kind
/// and the `dyn Trait` type.
pub fn trait_wrapper_info(ty: &Type) -> Option<(TraitWrapperKind, &Type)> {
	let Type::Path(path) = ty else { return None };
	let segment = path.path.segments.last()?;
	let kind = match segment.ident.to_string().as_str() {
		"Box" => TraitWrapperKind::Box,
		"Arc" => TraitWrapperKind::Arc,
		_ => return None,
	};
	let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
		return None;
	};
	let syn::GenericArgument::Type(inner) = args.args.first()? else {
		return None;
	};
	match inner {
		Type::TraitObject(_) => Some((kind, inner)),
		_ => None,
	}
}

/// Extract trait bounds from a `dyn Trait` type for use in `impl` position.
/// Strips the `dyn` keyword, ie `dyn Handler + Send` becomes `Handler + Send`.
pub fn trait_bounds_tokens(ty: &Type) -> Option<TokenStream> {
	if let Type::TraitObject(obj) = ty {
		let bounds = &obj.bounds;
		Some(quote! { #bounds })
	} else {
		None
	}
}

/// Returns true if the type should automatically use `impl Into<T>`.
/// Covers `String` and `Cow<'_, …>` types.
pub fn is_auto_into_type(ty: &syn::Type) -> bool {
	let syn::Type::Path(path) = ty else {
		return false;
	};
	let Some(seg) = path.path.segments.last() else {
		return false;
	};
	matches!(seg.ident.to_string().as_str(), "String" | "Cow" | "SmolStr")
}

/// Returns `true` for primitive types that implement `Copy`:
/// `bool`, `char`, all integer and float primitives.
/// Used to automatically return by value instead of by reference in the `Get` derive.
pub fn is_primitive_copy_type(ty: &syn::Type) -> bool {
	let syn::Type::Path(path) = ty else {
		return false;
	};
	if path.qself.is_some() {
		return false;
	}
	let Some(seg) = path.path.segments.last() else {
		return false;
	};
	// only bare primitive names, no generics
	if !matches!(seg.arguments, syn::PathArguments::None) {
		return false;
	}
	matches!(
		seg.ident.to_string().as_str(),
		"bool"
			| "char" | "f32"
			| "f64" | "i8"
			| "i16" | "i32"
			| "i64" | "i128"
			| "isize" | "u8"
			| "u16" | "u32"
			| "u64" | "u128"
			| "usize"
			// other known copy types
			| "Entity"
	)
}

/// For owned types that have a natural borrowed form, returns
/// `(return_type_tokens, accessor_tokens)`, ie:
/// - `String`  → `(&str,              as_str())`
/// - `PathBuf` → `(&::std::path::Path, as_path())`
/// - `OsString`→ `(&::std::ffi::OsStr, as_os_str())`
pub fn str_like_return(ty: &syn::Type) -> Option<(TokenStream, TokenStream)> {
	let syn::Type::Path(path) = ty else {
		return None;
	};
	let seg = path.path.segments.last()?;
	match seg.ident.to_string().as_str() {
		"String" => Some((quote! { &str }, quote! { as_str() })),
		"PathBuf" => {
			Some((quote! { &::std::path::Path }, quote! { as_path() }))
		}
		"OsString" => {
			Some((quote! { &::std::ffi::OsStr }, quote! { as_os_str() }))
		}
		_ => None,
	}
}

/// Resolve whether `impl Into<T>` should be used for a field, accounting for
/// auto-detection and explicit flags.
pub fn effective_use_into(ty: &syn::Type, config: &FieldConfig) -> bool {
	if config.not_into {
		return false;
	}
	config.use_into || is_auto_into_type(ty)
}

/// Generate the impl block for a getset derive. Called by each derive macro.
///
/// The callback receives the full [`syn::Field`] so derives can forward
/// doc attributes and inspect the type.
pub fn produce(
	input: &DeriveInput,
	attr_name: &str,
	generate_field: impl Fn(&syn::Field, &FieldConfig) -> TokenStream,
) -> syn::Result<TokenStream> {
	let syn::Data::Struct(data) = &input.data else {
		synbail!(input, "Getset derives only support structs");
	};
	let syn::Fields::Named(fields) = &data.fields else {
		synbail!(input, "Getset derives only support named fields");
	};

	let struct_config = parse_struct_config(&input.attrs, attr_name)?;
	if struct_config.ignore {
		return Ok(quote! {});
	}

	let mut methods = Vec::new();
	for field in &fields.named {
		let field_config =
			parse_field_config(&field.attrs, attr_name, &struct_config)?;
		if field_config.skip {
			continue;
		}
		methods.push(generate_field(field, &field_config));
	}

	let name = &input.ident;
	let (impl_generics, type_generics, where_clause) =
		input.generics.split_for_impl();

	Ok(quote! {
		impl #impl_generics #name #type_generics #where_clause {
			#(#methods)*
		}
	})
}


#[cfg(test)]
mod test {
	use super::*;
	use alloc::string::ToString;
	use quote::quote;
	use syn::DeriveInput;

	/// Extract named fields from a [`DeriveInput`] struct.
	fn named_fields(
		input: &DeriveInput,
	) -> &syn::punctuated::Punctuated<syn::Field, syn::Token![,]> {
		match &input.data {
			syn::Data::Struct(data) => match &data.fields {
				syn::Fields::Named(fields) => &fields.named,
				_ => unreachable!(),
			},
			_ => unreachable!(),
		}
	}

	// -- Vis --

	#[test]
	fn vis_default_is_pub() {
		assert_eq!(Vis::default(), Vis::Pub);
	}

	#[test]
	fn vis_to_tokens() {
		assert_eq!(Vis::Private.to_tokens().to_string(), "");
		assert_eq!(Vis::PubCrate.to_tokens().to_string(), "pub (crate)");
		assert_eq!(Vis::PubSuper.to_tokens().to_string(), "pub (super)");
		assert_eq!(Vis::Pub.to_tokens().to_string(), "pub");
	}

	#[test]
	fn vis_parsing_valid() {
		let expr: syn::Expr = syn::parse_quote!(private);
		assert_eq!(parse_vis(&expr).unwrap(), Vis::Private);

		let expr: syn::Expr = syn::parse_quote!(pub_crate);
		assert_eq!(parse_vis(&expr).unwrap(), Vis::PubCrate);

		let expr: syn::Expr = syn::parse_quote!(pub_super);
		assert_eq!(parse_vis(&expr).unwrap(), Vis::PubSuper);

		// `pub` is a keyword that cannot be a `syn::Expr`;
		// `Vis::Pub` is the default so it never needs explicit specification.
	}

	#[test]
	fn vis_parsing_invalid_ident() {
		let expr: syn::Expr = syn::parse_quote!(banana);
		assert!(parse_vis(&expr).is_err());
	}

	#[test]
	fn vis_parsing_invalid_expr() {
		let expr: syn::Expr = syn::parse_quote!(42);
		assert!(parse_vis(&expr).is_err());
	}

	// -- StructConfig --

	#[test]
	fn struct_config_defaults_when_no_attr() {
		let input: DeriveInput = syn::parse_quote! {
			struct Foo { bar: String }
		};
		let config = parse_struct_config(&input.attrs, "get").unwrap();
		assert_eq!(config.vis, Vis::Pub);
		assert_eq!(config.return_type, GetReturnType::Ref);
		assert!(!config.unwrap_option);
		assert!(!config.unwrap_trait);
	}

	#[test]
	fn struct_config_ignores_unrelated_attrs() {
		let input: DeriveInput = syn::parse_quote! {
			#[set(vis = private)]
			struct Foo { bar: String }
		};
		let config = parse_struct_config(&input.attrs, "get").unwrap();
		assert_eq!(config.vis, Vis::Pub);
	}

	#[test]
	fn struct_config_vis_and_clone() {
		let input: DeriveInput = syn::parse_quote! {
			#[get(vis = pub_crate, clone)]
			struct Foo { bar: String }
		};
		let config = parse_struct_config(&input.attrs, "get").unwrap();
		assert_eq!(config.vis, Vis::PubCrate);
		assert_eq!(config.return_type, GetReturnType::Clone);
		assert!(!config.unwrap_option);
	}

	#[test]
	fn struct_config_copy_and_unwrap_trait() {
		let input: DeriveInput = syn::parse_quote! {
			#[get(copy, unwrap_trait)]
			struct Foo { bar: u32 }
		};
		let config = parse_struct_config(&input.attrs, "get").unwrap();
		assert_eq!(config.return_type, GetReturnType::Copy);
		assert!(config.unwrap_trait);
	}

	#[test]
	fn struct_config_unwrap_option() {
		let input: DeriveInput = syn::parse_quote! {
			#[set(unwrap_option)]
			struct Foo { bar: Option<String> }
		};
		let config = parse_struct_config(&input.attrs, "set").unwrap();
		assert!(config.unwrap_option);
	}

	#[test]
	fn struct_config_ignore() {
		let input: DeriveInput = syn::parse_quote! {
			#[set(ignore)]
			struct Foo { bar: String }
		};
		let config = parse_struct_config(&input.attrs, "set").unwrap();
		assert!(config.ignore);
	}

	#[test]
	fn struct_config_invalid_key() {
		let input: DeriveInput = syn::parse_quote! {
			#[get(banana)]
			struct Foo { bar: String }
		};
		assert!(parse_struct_config(&input.attrs, "get").is_err());
	}

	// -- FieldConfig --

	#[test]
	fn field_config_inherits_defaults() {
		let defaults = StructConfig {
			vis: Vis::PubCrate,
			return_type: GetReturnType::Clone,
			unwrap_option: true,
			unwrap_trait: false,
			ignore: false,
		};
		let input: DeriveInput = syn::parse_quote! {
			struct Foo { bar: String }
		};
		let fields = named_fields(&input);
		let config =
			parse_field_config(&fields[0].attrs, "get", &defaults).unwrap();
		assert_eq!(config.vis, Vis::PubCrate);
		assert_eq!(config.return_type, GetReturnType::Clone);
		assert!(config.unwrap_option);
		assert!(!config.unwrap_trait);
		assert!(!config.skip);
	}

	#[test]
	fn field_config_overrides_vis_and_return_type() {
		let defaults = StructConfig::default();
		let input: DeriveInput = syn::parse_quote! {
			struct Foo {
				#[get(vis = private, copy, unwrap_trait)]
				bar: u32,
			}
		};
		let fields = named_fields(&input);
		let config =
			parse_field_config(&fields[0].attrs, "get", &defaults).unwrap();
		assert_eq!(config.vis, Vis::Private);
		assert_eq!(config.return_type, GetReturnType::Copy);
		assert!(config.unwrap_trait);
		assert!(!config.skip);
	}

	#[test]
	fn field_config_skip() {
		let defaults = StructConfig::default();
		let input: DeriveInput = syn::parse_quote! {
			struct Foo {
				#[get(skip)]
				bar: String,
			}
		};
		let fields = named_fields(&input);
		let config =
			parse_field_config(&fields[0].attrs, "get", &defaults).unwrap();
		assert!(config.skip);
	}

	#[test]
	fn field_config_invalid_key() {
		let defaults = StructConfig::default();
		let input: DeriveInput = syn::parse_quote! {
			struct Foo {
				#[get(foobar)]
				bar: String,
			}
		};
		let fields = named_fields(&input);
		assert!(
			parse_field_config(&fields[0].attrs, "get", &defaults).is_err()
		);
	}

	// -- option_inner_type --

	#[test]
	fn field_config_option_alias() {
		let defaults = StructConfig::default();
		let input: DeriveInput = syn::parse_quote! {
			struct Foo {
				#[set(option)]
				bar: Option<String>,
			}
		};
		let fields = named_fields(&input);
		let config =
			parse_field_config(&fields[0].attrs, "set", &defaults).unwrap();
		assert!(config.unwrap_option);
	}

	#[test]
	fn produce_ignore_generates_nothing() {
		let input: DeriveInput = syn::parse_quote! {
			#[get(ignore)]
			struct Foo {
				bar: String,
				bazz: u32,
			}
		};
		let result = produce(&input, "get", |_field, _config| {
			quote! { should_not_appear }
		})
		.unwrap()
		.to_string();
		assert!(result.is_empty());
	}

	// -- Type helpers --

	#[test]
	fn option_inner_extracts_type() {
		let ty: Type = syn::parse_quote!(Option<String>);
		let inner = option_inner_type(&ty).unwrap();
		let expected: Type = syn::parse_quote!(String);
		assert_eq!(quote!(#inner).to_string(), quote!(#expected).to_string());
	}

	#[test]
	fn option_inner_nested() {
		let ty: Type = syn::parse_quote!(Option<Vec<u32>>);
		let inner = option_inner_type(&ty).unwrap();
		let expected: Type = syn::parse_quote!(Vec<u32>);
		assert_eq!(quote!(#inner).to_string(), quote!(#expected).to_string());
	}

	#[test]
	fn option_inner_returns_none_for_non_option() {
		let ty: Type = syn::parse_quote!(String);
		assert!(option_inner_type(&ty).is_none());

		let ty: Type = syn::parse_quote!(Vec<u32>);
		assert!(option_inner_type(&ty).is_none());
	}

	// -- trait_wrapper_info --

	#[test]
	fn trait_wrapper_detects_box_dyn() {
		let ty: Type = syn::parse_quote!(Box<dyn MyTrait>);
		let (kind, inner) = trait_wrapper_info(&ty).unwrap();
		assert_eq!(kind, TraitWrapperKind::Box);
		assert_eq!(quote!(#inner).to_string(), "dyn MyTrait");
	}

	#[test]
	fn trait_wrapper_detects_arc_dyn() {
		let ty: Type = syn::parse_quote!(Arc<dyn MyTrait>);
		let (kind, inner) = trait_wrapper_info(&ty).unwrap();
		assert_eq!(kind, TraitWrapperKind::Arc);
		assert_eq!(quote!(#inner).to_string(), "dyn MyTrait");
	}

	#[test]
	fn trait_wrapper_ignores_non_dyn() {
		let ty: Type = syn::parse_quote!(Box<String>);
		assert!(trait_wrapper_info(&ty).is_none());
	}

	#[test]
	fn trait_wrapper_ignores_other_wrappers() {
		let ty: Type = syn::parse_quote!(Vec<dyn MyTrait>);
		assert!(trait_wrapper_info(&ty).is_none());
	}

	#[test]
	fn trait_wrapper_ignores_plain_types() {
		let ty: Type = syn::parse_quote!(String);
		assert!(trait_wrapper_info(&ty).is_none());
	}

	// -- trait_bounds_tokens --

	#[test]
	fn trait_bounds_strips_dyn() {
		let ty: Type = syn::parse_quote!(dyn MyTrait);
		let tokens = trait_bounds_tokens(&ty).unwrap();
		assert_eq!(tokens.to_string(), "MyTrait");
	}

	#[test]
	fn trait_bounds_multi() {
		let ty: Type = syn::parse_quote!(dyn MyTrait + Send);
		let tokens = trait_bounds_tokens(&ty).unwrap();
		assert_eq!(tokens.to_string(), "MyTrait + Send");
	}

	#[test]
	fn trait_bounds_returns_none_for_non_trait() {
		let ty: Type = syn::parse_quote!(String);
		assert!(trait_bounds_tokens(&ty).is_none());
	}

	// -- is_primitive_copy_type --

	#[test]
	fn primitive_copy_detects_bool() {
		let ty: Type = syn::parse_quote!(bool);
		assert!(is_primitive_copy_type(&ty));
	}

	#[test]
	fn primitive_copy_detects_numerics() {
		for src in &[
			"i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32",
			"u64", "u128", "usize", "f32", "f64", "char",
		] {
			let ty: Type = syn::parse_str(src).unwrap();
			assert!(is_primitive_copy_type(&ty), "{src} should be copy");
		}
	}

	#[test]
	fn primitive_copy_rejects_string() {
		let ty: Type = syn::parse_quote!(String);
		assert!(!is_primitive_copy_type(&ty));
	}

	#[test]
	fn primitive_copy_rejects_generic() {
		// Vec<u8> has generics, so not a bare primitive
		let ty: Type = syn::parse_quote!(Vec<u8>);
		assert!(!is_primitive_copy_type(&ty));
	}

	#[test]
	fn primitive_copy_rejects_qualified() {
		let ty: Type = syn::parse_quote!(<Foo as Bar>::i32);
		assert!(!is_primitive_copy_type(&ty));
	}

	// -- str_like_return --

	#[test]
	fn str_like_string_returns_str() {
		let ty: Type = syn::parse_quote!(String);
		let (ret_ty, accessor) = str_like_return(&ty).unwrap();
		assert_eq!(ret_ty.to_string(), "& str");
		assert_eq!(accessor.to_string(), "as_str ()");
	}

	#[test]
	fn str_like_pathbuf_returns_path() {
		let ty: Type = syn::parse_quote!(PathBuf);
		let (ret_ty, accessor) = str_like_return(&ty).unwrap();
		assert!(ret_ty.to_string().contains("Path"));
		assert_eq!(accessor.to_string(), "as_path ()");
	}

	#[test]
	fn str_like_osstring_returns_osstr() {
		let ty: Type = syn::parse_quote!(OsString);
		let (ret_ty, accessor) = str_like_return(&ty).unwrap();
		assert!(ret_ty.to_string().contains("OsStr"));
		assert_eq!(accessor.to_string(), "as_os_str ()");
	}

	#[test]
	fn str_like_returns_none_for_unknown() {
		let ty: Type = syn::parse_quote!(Vec<u8>);
		assert!(str_like_return(&ty).is_none());

		let ty: Type = syn::parse_quote!(bool);
		assert!(str_like_return(&ty).is_none());
	}

	// -- produce --

	#[test]
	fn produce_basic_struct() {
		let input: DeriveInput = syn::parse_quote! {
			struct Foo {
				bar: String,
				bazz: u32,
			}
		};
		let result = produce(&input, "get", |field, config| {
			let ident = field.ident.as_ref().unwrap();
			let ty = &field.ty;
			let vis = config.vis.to_tokens();
			quote! { #vis fn #ident(&self) -> &#ty { &self.#ident } }
		})
		.unwrap()
		.to_string();
		assert!(result.contains("fn bar"));
		assert!(result.contains("fn bazz"));
	}

	#[test]
	fn produce_skips_fields() {
		let input: DeriveInput = syn::parse_quote! {
			struct Foo {
				#[get(skip)]
				bar: String,
				bazz: u32,
			}
		};
		let result = produce(&input, "get", |field, config| {
			let ident = field.ident.as_ref().unwrap();
			let ty = &field.ty;
			let vis = config.vis.to_tokens();
			quote! { #vis fn #ident(&self) -> &#ty { &self.#ident } }
		})
		.unwrap()
		.to_string();
		assert!(!result.contains("fn bar"));
		assert!(result.contains("fn bazz"));
	}

	#[test]
	fn produce_preserves_generics() {
		let input: DeriveInput = syn::parse_quote! {
			struct Foo<T: Clone> where T: Default {
				bar: T,
			}
		};
		let result = produce(&input, "get", |field, _config| {
			let ident = field.ident.as_ref().unwrap();
			let ty = &field.ty;
			quote! { pub fn #ident(&self) -> &#ty { &self.#ident } }
		})
		.unwrap()
		.to_string();
		assert!(result.contains("impl < T : Clone >"));
		assert!(result.contains("Foo < T >"));
		assert!(result.contains("T : Default"));
	}

	#[test]
	fn produce_applies_struct_level_vis() {
		let input: DeriveInput = syn::parse_quote! {
			#[get(vis = pub_crate)]
			struct Foo {
				bar: String,
			}
		};
		let result = produce(&input, "get", |field, config| {
			let ident = field.ident.as_ref().unwrap();
			let ty = &field.ty;
			let vis = config.vis.to_tokens();
			quote! { #vis fn #ident(&self) -> &#ty { &self.#ident } }
		})
		.unwrap()
		.to_string();
		assert!(result.contains("pub (crate)"));
	}

	#[test]
	fn produce_rejects_enum() {
		let input: DeriveInput = syn::parse_quote! {
			enum Foo { Bar, Bazz }
		};
		assert!(produce(&input, "get", |_, _| quote! {}).is_err());
	}

	#[test]
	fn produce_rejects_tuple_struct() {
		let input: DeriveInput = syn::parse_quote! {
			struct Foo(String, u32);
		};
		assert!(produce(&input, "get", |_, _| quote! {}).is_err());
	}

	#[test]
	fn produce_unit_struct_is_empty() {
		let input: DeriveInput = syn::parse_quote! {
			struct Foo {}
		};
		let result = produce(&input, "get", |field, _config| {
			let ident = field.ident.as_ref().unwrap();
			let ty = &field.ty;
			quote! { pub fn #ident(&self) -> &#ty { &self.#ident } }
		})
		.unwrap()
		.to_string();
		assert!(result.contains("impl Foo"));
		assert!(!result.contains("fn "));
	}
}
