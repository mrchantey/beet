//

use beet_common::node::AttributeLit;
use quote::ToTokens;

/// For a given [`syn::Lit`], return its most sensible string representation.
pub fn lit_to_attr(lit: &syn::Lit) -> AttributeLit {
	match lit {
		syn::Lit::Int(lit_int) => AttributeLit::Number(
			lit_int.base10_digits().parse::<f64>().unwrap(),
		),
		syn::Lit::Float(lit_float) => AttributeLit::Number(
			lit_float.base10_digits().parse::<f64>().unwrap(),
		),
		syn::Lit::Bool(lit_bool) => AttributeLit::Boolean(lit_bool.value()),
		syn::Lit::Str(lit_str) => AttributeLit::String(lit_str.value()),
		syn::Lit::ByteStr(lit_byte_str) => AttributeLit::String(
			String::from_utf8_lossy(&lit_byte_str.value()).into_owned(),
		),
		syn::Lit::Byte(lit_byte) => {
			AttributeLit::String(lit_byte.value().to_string())
		}
		syn::Lit::Char(lit_char) => {
			AttributeLit::String(lit_char.value().to_string())
		}
		syn::Lit::Verbatim(lit_verbatim) => {
			AttributeLit::String(lit_verbatim.to_string())
		}
		syn::Lit::CStr(lit_cstr) => {
			AttributeLit::String(lit_cstr.to_token_stream().to_string())
		}
		_ => unimplemented!(),
	}
}

pub fn expr_str(expr: syn::Expr) -> Option<String> {
	if let syn::Expr::Lit(expr_lit) = expr {
		if let syn::Lit::Str(lit_str) = &expr_lit.lit {
			return Some(lit_str.value());
		}
	}
	None
}
