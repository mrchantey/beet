use crate::prelude::*;
use beet_common::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use heck::ToUpperCamelCase;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use send_wrapper::SendWrapper;
use sweet::prelude::PipelineTarget;
use syn::Expr;
use syn::ExprClosure;
use syn::Ident;
use syn::Pat;
use syn::parse_quote;


/// [`SystemParam`] capable of finding all [`Attributes`] of a node,
/// collecting them into a [`TokenStream`]. The manner in shich they 
/// are added depends on whether the node is an [`ElementNode`] or a [`TemplateNode`].
#[rustfmt::skip]
#[derive(SystemParam)]
pub struct TokenizeAttributes<'w, 's> {
	_non_send: TempNonSendMarker<'w>,	
	attr_lits: Query<'w, 's, &'static AttributeLit>,
	elements: Query<'w, 's, Option<&'static Attributes>, With<ElementNode>>,
	templates: Query<'w,'s,
		(
			&'static NodeTag,
			Option<&'static ItemOf<NodeTag, SendWrapper<Span>>>,
			&'static ItemOf<TemplateNode,RustyTracker>, 
			Option<&'static Attributes>,
		),
		With<TemplateNode>,
	>,
	client_directives: Query<'w, 's, (), With<ClientIslandDirective>>,
	exprs: MaybeSpannedQuery<'w, 's, AttributeExpr>,
	keys: MaybeSpannedQuery<'w, 's, AttributeKeyExpr>,
	vals: MaybeSpannedQuery<'w, 's, AttributeValueExpr>,
}

impl TokenizeAttributes<'_, '_> {
	pub fn try_push_attributes(
		&self,
		try_combinator: impl Clone + Fn(Entity) -> Result<Option<TokenStream>>,
		items: &mut Vec<proc_macro2::TokenStream>,
		entity: Entity,
	) -> Result<()> {
		self.handle_element(try_combinator.clone(), items, entity)?;
		self.handle_template(try_combinator, items, entity)?;
		Ok(())
	}
	fn handle_element(
		&self,
		try_combinator: impl Fn(Entity) -> Result<Option<TokenStream>>,
		entity_components: &mut Vec<TokenStream>,
		entity: Entity,
	) -> Result<()> {
		let Ok(attributes) = self.elements.get(entity) else {
			return Ok(());
		};

		let mut attr_entities = Vec::new();

		if let Some(attrs) = attributes {
			for attr_entity in attrs.iter() {
				if let Some(event_func) =
					self.try_event_observer(attr_entity)?
				{
					// in the case of an event the value is an observer added to the parent
					entity_components.push(event_func);
					continue;
				}

				let mut attr_components = Vec::new();
				// blocks ie <span {Vec3::new()} />
				// inserted directly as an entity component
				if let Some(attr) =
					self.maybe_spanned_expr(attr_entity, &self.exprs)?
				{
					entity_components.push(quote! {#attr.into_node_bundle()});
				}

				if let Some(attr) =
					self.maybe_spanned_expr(attr_entity, &self.keys)?
				{
					attr_components.push(quote! {#attr.into_attr_key_bundle()});
				}
				if let Some(attr) =
					self.maybe_spanned_expr(attr_entity, &self.vals)?
				{
					attr_components.push(quote! {#attr.into_attr_val_bundle()});
				}
				if let Some(attr) = try_combinator(attr_entity)? {
					if self.keys.contains(attr_entity) {
						// if this attribute has a key, the combinator must be a value
						attr_components
							.push(quote! {#attr.into_attr_val_bundle()});
					} else {
						// otherwise the combinator is a block value, aka a component
						entity_components.push(attr);
					}
				}

				if attr_components.len() == 1 {
					attr_entities.push(attr_components.pop().unwrap());
				} else if !attr_components.is_empty() {
					attr_entities.push(quote! {
						(#(#attr_components),*)
					});
				}
			}
		}
		if !attr_entities.is_empty() {
			entity_components.push(quote! {
				related!(Attributes[
				#(#attr_entities),*
			])
					});
		}
		Ok(())
	}
	/// create a token stream for a [`SendWrapper<syn::Expr>`] expression which may be spanned
	fn maybe_spanned_expr<
		T: Component + std::ops::Deref<Target = SendWrapper<Expr>>,
	>(
		&self,
		entity: Entity,
		query: &MaybeSpannedQuery<T>,
	) -> Result<Option<Expr>> {
		if let Ok((item, span)) = query.get(entity) {
			let item = (***item).clone();
			if let Some(span) = span {
				Some(syn::parse_quote_spanned! { ***span =>
					#item
				})
			} else {
				Some(item)
			}
		} else {
			None
		}
		.xok()
	}

	/// If the attribute matches the requirements for an event observer,
	/// parse and return as an [`EntityObserver`].
	///
	/// ## Requirements
	/// - Key is a string literal starting with `on`
	/// - Value is not a string, (allows for verbatim js handlers)
	fn try_event_observer(
		&self,
		entity: Entity,
	) -> Result<Option<TokenStream>> {
		let Some(mut attr) = self.maybe_spanned_expr(entity, &self.vals)?
		else {
			return Ok(None);
		};


		let Ok(lit) = self.attr_lits.get(entity) else {
			return Ok(None);
		};
		// If value is a string literal, we shouldn't process it as an event handler,
		// to preserve onclick="some_js_function()"
		if lit.value.is_some() {
			return Ok(None);
		}

		let Some(suffix) = lit.key.strip_prefix("on") else {
			return Ok(None);
		};

		let span = self
			.keys
			.get(entity)
			.map(|s| s.1)
			.ok()
			.flatten()
			.map(|s| ***s)
			.unwrap_or(Span::call_site());

		let suffix = ToUpperCamelCase::to_upper_camel_case(suffix);

		let event_key = Ident::new(&format!("On{suffix}"), span);

		Self::try_insert_closure_type(&mut attr, &event_key);
		quote! {EntityObserver::new(#[allow(unused_braces)]#attr)}
			.xsome()
			.xok()
	}

	/// if the tokens are a closure or a block where the last statement is a closure,
	/// insert the matching [`Trigger`] type
	fn try_insert_closure_type(expr: &mut Expr, ident: &Ident) {
		fn process_closure(closure: &mut ExprClosure, ident: &Ident) {
			match closure.inputs.first_mut() {
				Some(first_param) => match &*first_param {
					Pat::Type(_) => {
						// Already has type annotation, leave as is
					}
					pat => {
						let pat_clone = pat.clone();
						// insert type
						*first_param = Pat::Type(
							parse_quote! {#pat_clone:Trigger<#ident>},
						);
					}
				},
				None => {
					// If no parameters, add one with discard name
					closure
						.inputs
						.push(Pat::Type(parse_quote!(_:Trigger<#ident>)));
				}
			};
		}

		match expr {
			Expr::Closure(closure) => {
				process_closure(closure, ident);
			}
			Expr::Block(block) => {
				// Handle the case where a block's last statement is a closure
				if let Some(last_stmt) = block.block.stmts.last_mut() {
					if let syn::Stmt::Expr(Expr::Closure(closure), _) =
						last_stmt
					{
						process_closure(closure, ident);
					}
				}
				// Block doesn't end with a closure, return unchanged
			}
			_ => {
				// Not a closure or block, unchanged
			}
		}
	}

	// currently construct using a custom builder pattern but we can
	// replace that with the EntityPatch system when that arrives
	#[allow(unused)]
	fn handle_template(
		&self,
		build_tokens: impl Fn(Entity) -> Result<Option<TokenStream>>,
		entity_components: &mut Vec<TokenStream>,
		entity: Entity,
	) -> Result<()> {
		let Ok((node_tag, node_tag_span, tracker, attributes)) =
			self.templates.get(entity)
		else {
			return Ok(());
		};

		let mut prop_assignments = Vec::new();

		if let Some(attrs) = attributes {
			for attr_entity in attrs.iter() {
				if let Some(attr) =
					self.maybe_spanned_expr(attr_entity, &self.exprs)?
				{
					entity_components.push(quote! {#attr.into_node_bundle()});
				}
				let combinator_attr = build_tokens(attr_entity)?;

				if let Some(key) =
					self.maybe_spanned_expr(attr_entity, &self.keys)?
					&& let Some(key) = expr_to_ident(&key)
				{
					if let Some(val) = combinator_attr {
						// first check if there was a combinator value
						prop_assignments.push(quote! {.#key(#val)});
					} else {
						// otherwise check if theres a regular value
						let value = self
							.maybe_spanned_expr(attr_entity, &self.vals)?
							.unwrap_or_else(|| {
								// finally no value means a bool flag
								syn::parse_quote! {true}
							});
						prop_assignments.push(quote! {.#key(#value)});
					}
				} else if let Some(value) = combinator_attr {
					// if it doesnt have a key, the combinator must be a block value
					entity_components.push(value);
				}
			}
		}

		let template_ident = Ident::new(
			&node_tag.as_str(),
			node_tag_span.map(|s| ***s).unwrap_or(Span::call_site()),
		);

		// we create an inner tuple, so that we can define the template
		// and reuuse it for serialization
		let mut inner_items = Vec::new();
		if self.client_directives.contains(entity) {
			inner_items.push(quote! {
				#[cfg(not(target_arch = "wasm32"))]
				{TemplateSerde::new(&template)}
				#[cfg(target_arch = "wasm32")]
				{()}
			});
		}
		// the output of a template is *children!*, ie the template is a fragment.
		// this is important to avoid duplicate components like NodeTag
		inner_items.push(
			quote! {TemplateRoot::spawn(Spawn(template.into_node_bundle()))},
		);
		entity_components.push(tracker.into_custom_token_stream());

		entity_components.push(quote! {{
			let template = <#template_ident as Props>::Builder::default()
					#(#prop_assignments)*
					.build();
			#[allow(unused_braces)]
			(#(#inner_items),*)
		}});
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use proc_macro2::Span;
	use proc_macro2::TokenStream;
	use quote::ToTokens;
	use quote::quote;
	use sweet::prelude::*;
	use syn::Ident;

	fn parse(val: TokenStream) -> String {
		let mut val = syn::parse2(val).unwrap();
		TokenizeAttributes::try_insert_closure_type(
			&mut val,
			&Ident::new("OnClick", Span::call_site()),
		);
		val.to_token_stream().to_string()
	}

	#[test]
	fn insert_closure_type() {
		// leaves typed
		parse(quote! { |_: Trigger<WeirdType>| {} })
			.xpect()
			.to_be(quote! { |_: Trigger<WeirdType>| {} }.to_string());
		// inserts inferred
		parse(quote! { |foo| {} })
			.xpect()
			.to_be(quote! { |foo: Trigger<OnClick>| {} }.to_string());
		// inserts discard for empty
		parse(quote! { || {} })
			.xpect()
			.to_be(quote! { |_: Trigger<OnClick>| {} }.to_string());
		// handles blocks
		parse(quote! { {|| {}} })
			.xpect()
			.to_be(quote! { {|_: Trigger<OnClick>| {}} }.to_string());
	}
}
