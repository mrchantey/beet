//! Token-stream helpers shared by proc macros and codegen.
use alloc::vec::Vec;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

/// Bevy's bounded `related!` / tuple `Bundle` arity limit.
const BOUNDED_MAX: usize = 12;

/// Emit a `related!` call when items fit a bevy tuple, otherwise fall back to
/// `spawn_with::<R, _>` so an arbitrary number of children can be spawned.
pub fn unbounded_related(
	relation: &Ident,
	related: Vec<TokenStream>,
) -> TokenStream {
	if related.len() <= BOUNDED_MAX {
		quote! { related!{ #relation [#(#related),*] } }
	} else {
		quote! { spawn_with::<#relation, _>(move |parent| {
			#(parent.spawn(#related);)*
		}) }
	}
}

/// Emit a tuple `Bundle` when items fit a bevy tuple, otherwise fall back to
/// `OnSpawn` so an arbitrary number of components can be inserted.
pub fn unbounded_bundle(items: Vec<TokenStream>) -> TokenStream {
	if items.is_empty() {
		quote! { () }
	} else if items.len() == 1 {
		items.into_iter().next().unwrap()
	} else if items.len() <= BOUNDED_MAX {
		quote! { (#(#items),*) }
	} else {
		quote! { OnSpawn::new(move |entity| {
			#(entity.insert(#items);)*
		}) }
	}
}

/// Build an [`Ident`] from `key`, escaping any rust reserved keyword as a raw
/// identifier so it can be used as a struct field, variable, etc.
pub fn non_reserved_key(key: &str, span: Span) -> Ident {
	if let Some(inner) = key.strip_prefix("r#") {
		return Ident::new_raw(inner, span);
	}
	if is_reserved_keyword(key) {
		Ident::new_raw(key, span)
	} else {
		Ident::new(key, span)
	}
}

/// Rust reserved keywords as defined in
/// <https://doc.rust-lang.org/reference/keywords.html>, plus reserved-for-future
/// and weak keywords.
fn is_reserved_keyword(key: &str) -> bool {
	matches!(
		key,
		// all editions
		"as" | "break"
			| "const" | "continue"
			| "crate" | "else" | "enum"
			| "extern" | "false" | "fn"
			| "for" | "if" | "impl"
			| "in" | "let" | "loop"
			| "match" | "mod" | "move"
			| "mut" | "pub" | "ref"
			| "return" | "self" | "Self"
			| "static" | "struct" | "super"
			| "trait" | "true" | "type"
			| "unsafe" | "use" | "where"
			| "while"
			// 2018 edition
			| "async" | "await" | "dyn"
			// reserved for future use
			| "abstract" | "become" | "box"
			| "do" | "final" | "macro"
			| "override" | "priv" | "typeof"
			| "unsized" | "virtual" | "yield"
			// reserved 2018
			| "try"
			// reserved 2024
			| "gen"
			// weak keywords
			| "'static" | "macro_rules" | "raw" | "safe" | "union"
	)
}

#[cfg(test)]
mod test {
	use super::*;
	use alloc::string::ToString;
	use alloc::vec;

	#[test]
	fn non_reserved_key_passthrough() {
		let span = Span::call_site();
		assert_eq!(non_reserved_key("foo", span).to_string(), "foo");
		assert_eq!(non_reserved_key("type", span).to_string(), "r#type");
		assert_eq!(non_reserved_key("r#type", span).to_string(), "r#type");
	}

	#[test]
	fn unbounded_bundle_arities() {
		assert_eq!(unbounded_bundle(Vec::new()).to_string(), "()");
		let one = vec![quote!(Foo)];
		assert_eq!(unbounded_bundle(one).to_string(), "Foo");
		let two = vec![quote!(Foo), quote!(Bar)];
		assert_eq!(unbounded_bundle(two).to_string(), "(Foo , Bar)");
	}

	#[test]
	fn unbounded_related_arities() {
		let relation = Ident::new("Children", Span::call_site());
		let two = vec![quote!(Foo), quote!(Bar)];
		assert_eq!(
			unbounded_related(&relation, two).to_string(),
			"related ! { Children [Foo , Bar] }"
		);
	}
}
