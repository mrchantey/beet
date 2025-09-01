use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;
use syn::FnArg;
use syn::ItemFn;
use syn::Stmt;
use syn::parse_macro_input;
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{
	self,
};


pub fn async_system(
	attr: proc_macro::TokenStream,
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let is_local = {
		let s = attr.to_string();
		s.split(',').any(|p| p.trim() == "local")
	};
	let input = parse_macro_input!(input as ItemFn);
	parse(input, is_local)
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}

fn is_top_level_await_stmt(stmt: &Stmt) -> bool {
	match stmt {
		Stmt::Local(local) => {
			if let Some(init) = &local.init {
				matches!(&*init.expr, Expr::Await(_))
			} else {
				false
			}
		}
		Stmt::Expr(expr, _) => matches!(expr, Expr::Await(_)),
		_ => false,
	}
}

fn build_nested(
	stmts: &[Stmt],
	closure_params: &Punctuated<FnArg, Comma>,
	spawn_method: &Ident,
) -> TokenStream {
	if let Some((idx, _)) = stmts
		.iter()
		.enumerate()
		.find(|(_, s)| is_top_level_await_stmt(s))
	{
		let before = &stmts[..idx];
		let await_stmt = &stmts[idx];
		let after = &stmts[idx + 1..];
		let inner = build_nested(after, closure_params, spawn_method);
		quote! {
			#(#before)*
			spawn_async.#spawn_method(async move {
				#await_stmt
				move |#closure_params| {
					#inner
				}
			});
		}
	} else {
		quote! { #(#stmts)* }
	}
}

fn parse(mut input: ItemFn, is_local: bool) -> syn::Result<TokenStream> {
	let mut sig = input.sig.clone();
	// Remove async from the top-level function
	sig.asyncness = None;

	// Prepend SpawnAsync to system params
	let mut new_inputs: Punctuated<FnArg, Comma> = Punctuated::new();
	new_inputs.push(parse_quote!(mut spawn_async: SpawnAsync));
	for arg in sig.inputs.clone() {
		new_inputs.push(arg);
	}
	sig.inputs = new_inputs;

	let closure_params = sig.inputs.clone();

	let spawn_method = if is_local {
		Ident::new("spawn_and_run_async_local", Span::call_site())
	} else {
		Ident::new("spawn_and_run_async", Span::call_site())
	};

	let body = build_nested(&input.block.stmts, &closure_params, &spawn_method);
	let attrs = input.attrs;
	let vis = input.vis;
	Ok(quote! {
		#(#attrs)*
		#vis #sig {
			#body
		}
	})
}
