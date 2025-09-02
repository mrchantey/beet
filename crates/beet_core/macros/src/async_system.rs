use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::quote;
use syn;
use syn::Expr;
use syn::FnArg;
use syn::ItemFn;
use syn::Stmt;
use syn::parse_macro_input;
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::token::Comma;

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


#[derive(Clone)]
struct Parser {
	closure_params: Punctuated<FnArg, Comma>,
	spawn_method: Ident,
	stream_method: Ident,
	ret_sender: Option<Ident>,
	world_ident: Option<Ident>,
	// Whether the top-level return type is a Result<...>
	ret_is_result: bool,
	// Are we generating inside a spawned closure
	in_closure: bool,
}


fn parse(input: ItemFn, is_local: bool) -> syn::Result<TokenStream> {
	let mut sig = input.sig;
	// Remove async from the top-level function
	sig.asyncness = None;

	// Determine if a &mut World parameter exists; if so, derive __async_commands from it.
	let world_ident: Option<Ident> = sig.inputs.iter().find_map(|arg| {
		if let FnArg::Typed(pat_ty) = arg {
			if let syn::Type::Reference(tyref) = &*pat_ty.ty {
				if tyref.mutability.is_some() {
					if let syn::Type::Path(tp) = &*tyref.elem {
						if tp.qself.is_none() {
							if let Some(seg) = tp.path.segments.last() {
								if seg.ident == "World" {
									if let syn::Pat::Ident(pat_ident) =
										&*pat_ty.pat
									{
										return Some(pat_ident.ident.clone());
									}
								}
							}
						}
					}
				}
			}
		}
		None
	});

	// Determine if the function is an observer (first param is Trigger<...>)
	let is_observer: bool = sig
		.inputs
		.first()
		.and_then(|arg| {
			if let FnArg::Typed(pat_ty) = arg {
				if let syn::Type::Path(tp) = &*pat_ty.ty {
					if tp.qself.is_none() {
						if let Some(seg) = tp.path.segments.last() {
							return Some(seg.ident == "Trigger");
						}
					}
				}
			}
			None
		})
		.unwrap_or(false);

	// Insert AsyncCommands while preserving Trigger<T> as the first param for observers
	if world_ident.is_none() {
		let mut new_inputs: Punctuated<FnArg, Comma> = Punctuated::new();
		if is_observer {
			// Keep the first parameter (Trigger<...>) first
			if let Some(first) = sig.inputs.first().cloned() {
				new_inputs.push(first);
			}
			// Insert AsyncCommands after Trigger
			new_inputs.push(parse_quote!(mut __async_commands: AsyncCommands));
			// Then push the rest of the original params
			for arg in sig.inputs.iter().skip(1).cloned() {
				new_inputs.push(arg);
			}
		} else {
			// Non-observers: AsyncCommands is the first param
			new_inputs.push(parse_quote!(mut __async_commands: AsyncCommands));
			for arg in sig.inputs.clone() {
				new_inputs.push(arg);
			}
		}
		sig.inputs = new_inputs;
	}

	// Child closures never receive the Trigger<T>; drop it if this is an observer.
	let closure_params = if is_observer {
		let mut filtered: Punctuated<FnArg, Comma> = Punctuated::new();
		let mut iter = sig.inputs.clone().into_iter();
		let _ = iter.next(); // skip Trigger<...>
		for arg in iter {
			filtered.push(arg);
		}
		filtered
	} else {
		sig.inputs.clone()
	};

	let spawn_method: Ident = if is_local {
		syn::parse_quote!(spawn_and_run_local)
	} else {
		syn::parse_quote!(spawn_and_run)
	};

	// choose the streaming method name based on local vs non-local
	let stream_method: Ident = if is_local {
		syn::parse_quote!(spawn_for_each_stream_local)
	} else {
		syn::parse_quote!(spawn_for_each_stream)
	};

	// If the function has a return type, adapt the signature to return a Future
	// that resolves with the value sent over an async_channel receiver.
	let return_ty = match &sig.output {
		syn::ReturnType::Type(_, ty) => Some((*ty).clone()),
		_ => None,
	};

	let body = if let Some(ret_ty) = return_ty.clone() {
		// Change the signature to return a pinned boxed Future of the original return type.
		if is_local {
			sig.output = parse_quote!(-> ::std::pin::Pin<Box<dyn ::core::future::Future<Output = #ret_ty> + 'static>>);
		} else {
			sig.output = parse_quote!(-> ::std::pin::Pin<Box<dyn ::core::future::Future<Output = #ret_ty> + Send + 'static>>);
		}

		// Build nested body with awareness of a return-value sender.
		let ret_is_result = match &*ret_ty {
			syn::Type::Path(tp) if tp.qself.is_none() => tp
				.path
				.segments
				.last()
				.map(|s| s.ident == "Result")
				.unwrap_or(false),
			_ => false,
		};
		let parser = Parser {
			closure_params: closure_params.clone(),
			spawn_method: spawn_method.clone(),
			stream_method: stream_method.clone(),
			ret_sender: Some(syn::parse_quote!(__beet_return_tx)),
			world_ident: world_ident.clone(),
			ret_is_result,
			in_closure: false,
		};
		let nested = parser.build_nested(&input.block.stmts, true);

		quote! {
			let (__beet_return_tx, __beet_return_rx) = beet::exports::async_channel::bounded::<#ret_ty>(1);
			// Helper to produce the boxed future; used for normal exit and for early-exit on `?` before first await.
			let __beet_finish = || {
				#[allow(unused_imports)]
				use ::std::boxed::Box;
				#[allow(unused_imports)]
				use ::std::pin::Pin;
				#[allow(unused_mut, unused_variables)]
				{
					#[allow(unused_unsafe)]
					let __beet_boxed = ::std::boxed::Box::pin(async move {
						match __beet_return_rx.recv().await {
							Ok(v) => v,
							Err(_) => panic!("async_system return channel closed"),
						}
					});
					// Cast to the concrete pinned trait-object type expected by the signature.
					__beet_boxed as _
				}
			};
			#nested
			{
				__beet_finish()
			}
		}
	} else {
		let parser = Parser {
			closure_params: closure_params.clone(),
			spawn_method: spawn_method.clone(),
			stream_method: stream_method.clone(),
			ret_sender: None,
			world_ident: world_ident.clone(),
			ret_is_result: false,
			in_closure: false,
		};
		{
			let nested = parser.build_nested(&input.block.stmts, false);
			quote! {
				#nested
			}
		}
	};

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


impl Parser {
	// Helper: tokens that cause an early return depending on context.
	fn early_return_tokens(&self) -> TokenStream {
		if self.in_closure {
			quote! { return; }
		} else {
			quote! { return __beet_finish(); }
		}
	}

	// Helper: lower a `?` expression into a match that sends Err on failure and early-returns.
	fn lower_try(&self, inner: &Expr) -> TokenStream {
		if !(self.ret_sender.is_some() && self.ret_is_result) {
			quote! { #inner? }
		} else {
			let ret_tx = self.ret_sender.as_ref().unwrap();
			let __beet_early = self.early_return_tokens();
			quote! {
				{
					let __beet_try_tmp = #inner;
					match __beet_try_tmp {
						Ok(__beet_ok) => __beet_ok,
						Err(__beet_err) => {
							let _ = #ret_tx.try_send(Err(__beet_err));
							#__beet_early
						}
					}
				}
			}
		}
	}

	fn build_nested(&self, stmts: &[Stmt], allow_tail: bool) -> TokenStream {
		let closure_params = &self.closure_params;
		let spawn_method = &self.spawn_method;
		let stream_method = &self.stream_method;

		// Look for streaming loops or top-level awaits in this statement list.
		for (idx, stmt) in stmts.iter().enumerate() {
			// Handle streaming `while let ... = ...await { ... }`
			if let Some((pat, stream_expr, body_block)) =
				match_while_let_await(stmt)
			{
				let before = &stmts[..idx];
				let after = &stmts[idx + 1..];

				// Recursively process the loop body (it may contain awaits)
				let mut child = self.clone();
				child.in_closure = true;
				let body_inner = child.build_nested(&body_block.stmts, false);
				let after_inner = child.build_nested(after, allow_tail);

				// If returning a value, clone sender so closures capture clones.
				let pre_clone = self.ret_sender.as_ref().map(|ident| {
					quote! { let #ident = #ident.clone(); }
				});

				let before_inner = self.build_nested(before, false);
				let __async_commands_expr = self.async_commands_tokens();
				let inner_clone = self.ret_sender.as_ref().map(|ident| {
					quote! { let #ident = #ident.clone(); }
				});
				return quote! {
					#before_inner
					#pre_clone
					{
						let __beet_stream = beet::exports::futures_lite::StreamExt::map(#stream_expr, |__beet_item| Option::Some(__beet_item));
						let __beet_stream = beet::exports::futures_lite::StreamExt::chain(__beet_stream, beet::exports::futures_lite::stream::once(Option::None));
						#__async_commands_expr.#stream_method(__beet_stream, move |__beet_opt| {
							#inner_clone
							#[allow(unused_mut, unused_variables)]
							move |#closure_params| {
								match __beet_opt {
									Option::Some(#pat) => { #body_inner }
									Option::None => { #after_inner }
								}
							}
						});
					}
				};
			// Handle a top-level await statement
			} else if is_top_level_await_stmt(stmt) {
				let before = &stmts[..idx];
				let await_stmt = stmt;
				let after = &stmts[idx + 1..];
				let inner = {
					let mut __beet_child = self.clone();
					__beet_child.in_closure = true;
					__beet_child.build_nested(after, allow_tail)
				};

				let pre_clone = self.ret_sender.as_ref().map(|ident| {
					quote! { let #ident = #ident.clone(); }
				});

				let before_inner = self.build_nested(before, false);
				let __async_commands_expr = self.async_commands_tokens();
				return quote! {
					#before_inner
					#pre_clone
					#__async_commands_expr.#spawn_method(async move {
						#await_stmt
						#[allow(unused_mut, unused_variables)]
						move |#closure_params| {
							#inner
						}
					});
				};
			}
		}

		// Early-return and tail-expression handling when a return sender exists.
		if let Some(ret_tx) = self.ret_sender.as_ref() {
			// Explicit top-level `return ...;`
			for (idx, stmt) in stmts.iter().enumerate() {
				if let Stmt::Expr(syn::Expr::Return(ret_expr), _) = stmt {
					let before = &stmts[..idx];
					let ret_value = if let Some(expr) = &ret_expr.expr {
						quote! { #expr }
					} else {
						quote! { () }
					};
					let pre_clone = self.ret_sender.as_ref().map(|ident| {
						quote! { let #ident = #ident.clone(); }
					});
					let before_inner = self.build_nested(before, false);
					return quote! {
						#before_inner
						#pre_clone
						#ret_tx.try_send(#ret_value).ok();
					};
				}
			}

			// Tail expression: only send at the terminal continuation of the top-level function.
			if allow_tail {
				if let Some(Stmt::Expr(expr, None)) = stmts.last() {
					let before = &stmts[..stmts.len().saturating_sub(1)];
					let before_inner = self.build_nested(before, false);
					// If the tail expression is a try (`expr?`) and we're returning Result,
					// convert it into a send of Ok(...) or early-send Err(...)
					if self.ret_is_result {
						if let Expr::Try(ex_try) = expr {
							let inner = &ex_try.expr;
							let __beet_early = if self.in_closure {
								quote! { return; }
							} else {
								quote! { return __beet_finish(); }
							};
							return quote! {
								#before_inner
								{
									let __beet_try_tmp = #inner;
									match __beet_try_tmp {
										Ok(__beet_ok) => {
											let _ = #ret_tx.try_send(__beet_ok);
										}
										Err(__beet_err) => {
											let _ = #ret_tx.try_send(Err(__beet_err));
											#__beet_early
										}
									}
								}
							};
						}
					}
					return quote! {
						#before_inner
						#ret_tx.try_send(#expr).ok();
					};
				}
			}
		}

		// Fallback: rebuild nested constructs (blocks/ifs/loops) so inner awaits/streams/returns are handled.
		let rebuilt: Vec<TokenStream> = stmts
			.iter()
			.map(|s| {
				self.rebuild_stmt(s, allow_tail)
					.unwrap_or_else(|| quote! { #s })
			})
			.collect();
		quote! { #(#rebuilt)* }
	}

	// Rebuild a statement if it contains a nested block/if/loop that may include awaits/returns.
	fn rebuild_stmt(
		&self,
		stmt: &Stmt,
		allow_tail: bool,
	) -> Option<TokenStream> {
		match stmt {
			// Handle top-level try expressions (e.g., `foo()?;`) in a unified way.
			Stmt::Expr(Expr::Try(ex_try), semi) => {
				if self.ret_sender.is_some() && self.ret_is_result {
					let lowered = self.lower_try(&ex_try.expr);
					if semi.is_some() {
						Some(quote! { #lowered; })
					} else {
						Some(quote! { #lowered })
					}
				} else {
					None
				}
			}
			// Handle let-initializer try expressions, including implicit diverge `?`.
			Stmt::Local(local) => {
				if let Some(init) = &local.init {
					if self.ret_sender.is_some() && self.ret_is_result {
						let has_qmark = matches!(&*init.expr, Expr::Try(_))
							|| init.diverge.is_some();
						if has_qmark {
							let pat = &local.pat;
							let inner_expr = match &*init.expr {
								Expr::Try(expr_try) => &expr_try.expr,
								_ => &init.expr,
							};
							let lowered = self.lower_try(inner_expr);
							return Some(quote! {
								let #pat = #lowered;
							});
						}
					}
				}
				None
			}
			Stmt::Expr(Expr::Block(b), semi) => {
				let inner = self.build_block(&b.block, allow_tail);
				if semi.is_some() {
					Some(quote! { { #inner }; })
				} else {
					Some(quote! { { #inner } })
				}
			}
			Stmt::Expr(Expr::If(ife), semi) => {
				let toks = self.build_if(ife, allow_tail);
				if semi.is_some() {
					Some(quote! { #toks; })
				} else {
					Some(quote! { #toks })
				}
			}
			// Non-streaming while loops: rebuild their body.
			Stmt::Expr(Expr::While(w), semi) => {
				if match_while_let_await(stmt).is_none() {
					let body = self.build_nested(&w.body.stmts, false);
					let cond = &w.cond;
					if semi.is_some() {
						Some(quote! { while #cond { #body }; })
					} else {
						Some(quote! { while #cond { #body } })
					}
				} else {
					None
				}
			}
			// Generic loop with nested awaits.
			Stmt::Expr(Expr::Loop(lp), semi) => {
				let body = self.build_nested(&lp.body.stmts, false);
				if semi.is_some() {
					Some(quote! { loop { #body }; })
				} else {
					Some(quote! { loop { #body } })
				}
			}
			// For loops with nested awaits.
			Stmt::Expr(Expr::ForLoop(fl), semi) => {
				let pat = &fl.pat;
				let expr = &fl.expr;
				let body = self.build_nested(&fl.body.stmts, false);
				if semi.is_some() {
					Some(quote! { for #pat in #expr { #body }; })
				} else {
					Some(quote! { for #pat in #expr { #body } })
				}
			}
			_ => None,
		}
	}

	// Rebuild a bare block using nested processing of its statements.
	fn build_block(&self, block: &syn::Block, allow_tail: bool) -> TokenStream {
		self.build_nested(&block.stmts, allow_tail)
	}

	// Rebuild an expression, handling try (`?`) when returning Result.
	fn rebuild_expr(&self, expr: &Expr) -> Option<TokenStream> {
		if self.ret_sender.is_some() && self.ret_is_result {
			match expr {
				Expr::Try(ex_try) => Some(self.lower_try(&ex_try.expr)),
				_ => None,
			}
		} else {
			None
		}
	}

	// Rebuild an `if` expression, recursing into then/else branches.
	fn build_if(&self, ife: &syn::ExprIf, _allow_tail: bool) -> TokenStream {
		let cond = &ife.cond;
		let cond_tokens =
			self.rebuild_expr(cond).unwrap_or_else(|| quote! { #cond });
		let then_inner = self.build_nested(&ife.then_branch.stmts, false);
		if let Some((_, else_expr)) = &ife.else_branch {
			match else_expr.as_ref() {
				Expr::If(else_if) => {
					let else_tokens = self.build_if(else_if, false);
					quote! { if #cond_tokens { #then_inner } else #else_tokens }
				}
				Expr::Block(else_block) => {
					let else_inner =
						self.build_nested(&else_block.block.stmts, false);
					quote! { if #cond_tokens { #then_inner } else { #else_inner } }
				}
				other => {
					quote! { if #cond_tokens { #then_inner } else { #other } }
				}
			}
		} else {
			quote! { if #cond_tokens { #then_inner } }
		}
	}
	fn async_commands_tokens(&self) -> TokenStream {
		if let Some(world_ident) = &self.world_ident {
			quote! { AsyncCommands{ commands: #world_ident.commands()} }
		} else {
			quote! { __async_commands }
		}
	}
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
