use proc_macro2::Ident;
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
		Stmt::Expr(Expr::Await(_), _) => true,
		_ => false,
	}
}

fn parse(input: ItemFn, is_local: bool) -> syn::Result<TokenStream> {
	let mut sig = input.sig;
	// Remove async from the top-level function
	sig.asyncness = None;

	// Prepend AsyncCommands to system params
	let mut new_inputs: Punctuated<FnArg, Comma> = Punctuated::new();
	new_inputs.push(parse_quote!(mut __async_commands: AsyncCommands));
	for arg in sig.inputs.clone() {
		new_inputs.push(arg);
	}
	sig.inputs = new_inputs;

	let closure_params = sig.inputs.clone();

	let spawn_method = if is_local {
		syn::parse_quote!(spawn_and_run)
	} else {
		syn::parse_quote!(spawn_and_run_local)
	};

	let body = build_nested(&input.block.stmts, &closure_params, &spawn_method);
	let attrs = input.attrs;
	let vis = input.vis;
	Ok(quote! {
		#(#attrs)*
		#[allow(unused_mut, unused_variables)]
		#vis #sig {
			#body
		}
	})
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
			__async_commands.#spawn_method(async move {
				#await_stmt
				#[allow(unused_mut, unused_variables)]
				move |#closure_params| {
					#inner
				}
			});
		}
	} else {
		quote! { #(#stmts)* }
	}
}



#[cfg(test)]
mod test {
	use super::parse;
	use sweet::prelude::*;

	#[test]
	fn async_system() {
		parse(
			syn::parse_quote! {
				async fn my_system(mut commands: Commands, mut query: Query<&mut Name>) {
					let stmt1 = 0;
					let stmt2 = stmt1.await;
					let stmt3 = 0;
					let stmt4 = stmt3.await;
					println!("query: {}", query);
				}
			},
			false,
		)
		.unwrap()
		.xpect()
		.to_be_snapshot();
	}
}
