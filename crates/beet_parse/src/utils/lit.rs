//
use beet_dom::prelude::*;
use bevy::ecs::system::EntityCommands;
use quote::ToTokens;

/// For a given [`syn::Lit`], return its most sensible string representation.
pub fn insert_lit(entity: &mut EntityCommands, lit: &syn::Lit) {
	match lit {
		syn::Lit::Int(lit_int) => {
			entity.insert(
				lit_int
					.base10_digits()
					.parse::<f64>()
					.unwrap()
					.into_bundle(),
			);
		}
		syn::Lit::Float(lit_float) => {
			entity.insert(
				lit_float
					.base10_digits()
					.parse::<f64>()
					.unwrap()
					.into_bundle(),
			);
		}
		syn::Lit::Bool(lit_bool) => {
			entity.insert(lit_bool.value().into_bundle());
		}
		syn::Lit::Str(lit_str) => {
			entity.insert(lit_str.value().into_bundle());
		}
		syn::Lit::ByteStr(lit_byte_str) => {
			entity.insert(
				String::from_utf8_lossy(&lit_byte_str.value())
					.into_owned()
					.into_bundle(),
			);
		}
		syn::Lit::Byte(lit_byte) => {
			entity.insert(lit_byte.value().to_string().into_bundle());
		}
		syn::Lit::Char(lit_char) => {
			entity.insert(lit_char.value().to_string().into_bundle());
		}
		syn::Lit::Verbatim(lit_verbatim) => {
			entity.insert(lit_verbatim.to_string().into_bundle());
		}
		syn::Lit::CStr(lit_cstr) => {
			entity.insert(lit_cstr.to_token_stream().to_string().into_bundle());
		}
		_ => unimplemented!(),
	}
}

/// Extracts a string value from an expression if it's a string literal.
pub fn expr_str(expr: syn::Expr) -> Option<String> {
	if let syn::Expr::Lit(expr_lit) = expr {
		if let syn::Lit::Str(lit_str) = &expr_lit.lit {
			return Some(lit_str.value());
		}
	}
	None
}
