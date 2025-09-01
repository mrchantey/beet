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

/// Detects the simple pattern of a top-level await expression used as a
/// initializer or standalone expression.
///
/// Examples matched:
/// - `let x = something.await;`
/// - `something.await;`
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

/// Attempt to match a `while let PAT = EXPR.await { ... }` statement.
///
/// If matched, returns (pattern, stream_expr, body_block).
/// For typical async-stream usage this will be the `.next().await` pattern and
/// `stream_expr` will be the receiver of the `next()` call. We conservatively
/// only match when the awaited expression is a method call (e.g. `foo.next()`),
/// but this can be extended if other patterns are required.
fn match_while_let_await(
	stmt: &Stmt,
) -> Option<(&syn::Pat, &Expr, &syn::Block)> {
	// Match `while let <pat> = <await_expr> { <body> }`
	if let Stmt::Expr(Expr::While(ew), _) = stmt {
		// `ew.cond` is a Box<Expr>
		if let Expr::Let(expr_let) = &*ew.cond {
			// Check that the right-hand side is an await expression
			if let Expr::Await(await_expr) = &*expr_let.expr {
				// If the awaited expression is a method call (e.g., `stream.next()`),
				// use its receiver as the stream expression.
				if let Expr::MethodCall(method_call) = &*await_expr.base {
					let stream_expr = &*method_call.receiver;
					return Some((&expr_let.pat, stream_expr, &ew.body));
				}
				// Otherwise, fallback: use the whole awaited expression as the
				// stream expression. This is less ergonomic for streams, but
				// allows patterns like `while let Some(x) = foo.await {}` where
				// `foo` itself is some wrapper that yields Option repeatedly.
				else {
					let stream_expr = &*await_expr.base;
					return Some((&expr_let.pat, stream_expr, &ew.body));
				}
			}
		}
	}
	None
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

	// choose the streaming method name based on local vs non-local
	let stream_method: Ident = if is_local {
		syn::parse_quote!(spawn_for_each_stream_local)
	} else {
		syn::parse_quote!(spawn_for_each_stream)
	};

	let body = build_nested(
		&input.block.stmts,
		&closure_params,
		&spawn_method,
		&stream_method,
	);
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
	stream_method: &Ident,
) -> TokenStream {
	// Iterate over all statements and find the first one that is either a
	// streaming `while let ... = ...await { ... }` or a top-level await stmt.
	for (idx, stmt) in stmts.iter().enumerate() {
		// Handle streaming `while let ... = ...await { ... }`
		if let Some((pat, stream_expr, body_block)) =
			match_while_let_await(stmt)
		{
			let before = &stmts[..idx];
			let after = &stmts[idx + 1..];

			// Recursively process the loop body (it may contain awaits)
			let body_inner = build_nested(
				&body_block.stmts,
				closure_params,
				spawn_method,
				stream_method,
			);
			let after_inner = build_nested(
				after,
				closure_params,
				spawn_method,
				stream_method,
			);

			// Emit a single call to the streaming API.
			return quote! {
				#(#before)*
				__async_commands.#stream_method(#stream_expr, move |#pat| {
					#[allow(unused_mut, unused_variables)]
					move |#closure_params| {
						#body_inner
					}
				});
				#after_inner
			};
		}

		// Handle a top-level await statement
		if is_top_level_await_stmt(stmt) {
			let before = &stmts[..idx];
			let await_stmt = stmt;
			let after = &stmts[idx + 1..];
			let inner = build_nested(
				after,
				closure_params,
				spawn_method,
				stream_method,
			);

			return quote! {
				#(#before)*
				__async_commands.#spawn_method(async move {
					#await_stmt
					#[allow(unused_mut, unused_variables)]
					move |#closure_params| {
						#inner
					}
				});
			};
		}
	}

	// No special statements found; just return the original statements.
	quote! { #(#stmts)* }
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

	#[test]
	fn while_let_await_streaming() {
		parse(
			syn::parse_quote! {
				async fn my_streaming_system(mut commands: Commands, mut query: Query<&mut Name>) {
					let mut s = stream();
					while let Some(item) = s.next().await {
						let v = item.awaitable.await;
						println!("got item: {:?}", v);
						*query.single_mut().unwrap() = "updated".into();
					}
					println!("done");
				}
			},
			false,
		)
		.unwrap()
		.xpect()
		.to_be_snapshot();
	}
}
