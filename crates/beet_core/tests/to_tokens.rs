#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_core::as_beet::*;
use quote::quote;
use sweet::prelude::*;

#[test]
fn named() {
	#[derive(ToTokens)]
	struct Named {
		num: u32,
		string: String,
	}

	let foo = Named {
		num: 42,
		string: "Hello".to_string(),
	}
	.self_token_stream()
	.to_string();
	foo.xpect_eq(
		quote! {Named {
				num: 42u32,
				string: String::from("Hello")
			}
		}
		.to_string(),
	);
}

#[test]
fn unnamed() {
	#[derive(ToTokens)]
	struct Unnamed(u32, String);

	let foo = Unnamed(42, "Hello".to_string())
		.self_token_stream()
		.to_string();
	foo.xpect_eq(
		quote! {
			Unnamed(42u32, String::from("Hello"))
		}
		.to_string(),
	);
}

#[test]
fn nested_struct() {
	#[derive(ToTokens)]
	struct Named1 {
		value: u32,
		text: String,
	}

	#[derive(ToTokens)]
	struct Named2 {
		inner: Named1,
		flag: bool,
	}

	let nested = Named2 {
		inner: Named1 {
			value: 42,
			text: "Nested".to_string(),
		},
		flag: true,
	}
	.self_token_stream()
	.to_string();

	nested.xpect_eq(
		quote! {
			Named2 {
				inner: Named1 {
					value: 42u32,
					text: String::from("Nested")
				},
				flag: true
			}
		}
		.to_string(),
	);
}

#[test]
fn enum_variants() {
	#[derive(ToTokens)]
	enum TestEnum {
		Unit,
		Named { value: u32, text: String },
		Unnamed(bool, String),
	}

	let unit = TestEnum::Unit.self_token_stream().to_string();
	unit.xpect_eq(
		quote! {
			TestEnum::Unit
		}
		.to_string(),
	);

	let named = TestEnum::Named {
		value: 99,
		text: "Enum".to_string(),
	}
	.self_token_stream()
	.to_string();
	named.xpect_eq(
		quote! {
			TestEnum::Named {
				value: 99u32,
				text: String::from("Enum")
			}
		}
		.to_string(),
	);

	let unnamed = TestEnum::Unnamed(true, "Tuple".to_string())
		.self_token_stream()
		.to_string();
	unnamed.xpect_eq(
		quote! {
			TestEnum::Unnamed(true, String::from("Tuple"))
		}
		.to_string(),
	);
}
