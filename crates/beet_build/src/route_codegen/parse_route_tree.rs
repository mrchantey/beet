//! Static route tree generation for type-safe routing.
//!
//! This module generates a module hierarchy of path functions from route files,
//! enabling type-safe route path construction at compile time.

use crate::prelude::*;
use beet_core::prelude::*;
use heck::ToSnakeCase;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use syn::Ident;
use syn::Item;
use syn::ItemFn;
use syn::parse_quote;

/// Marks an entity for generating a static route tree.
///
/// When added to an entity, causes a `routes` module to be generated containing
/// path functions for every route in the entity's root. This enables type-safe
/// route path construction.
#[derive(Debug, Clone, Default, Component, Reflect)]
#[reflect(Component)]
pub struct StaticRouteTree;

/// Generates a tree of route path functions from [`RouteFileMethod`] entities.
///
/// This system creates a `routes` module containing functions that return
/// route paths, enabling type-safe route path construction.
///
/// # Example Output
///
/// For the following input files:
/// - `index.rs`
/// - `foo/bar/index.rs`
/// - `foo/bar/bazz.rs`
///
/// Generates:
/// ```ignore
/// mod routes {
///     pub fn index() -> &'static str { "/" }
///     mod foo {
///         mod bar {
///             pub fn index() -> &'static str { "/foo/bar" }
///             pub fn bazz() -> &'static str { "/foo/bar/bazz" }
///         }
///     }
/// }
/// ```
pub(crate) fn parse_route_tree(
	mut query: Populated<
		(Entity, &mut CodegenFile),
		(Added<CodegenFile>, With<StaticRouteTree>),
	>,
	collections: Query<(Entity, &RouteFileCollection)>,
	methods: Query<&RouteFileMethod>,
	parents: Query<&ChildOf>,
	children: Query<&Children>,
) {
	for (entity, mut codegen) in query.iter_mut() {
		let root = parents.root_ancestor(entity);


		let child_methods = children
			.iter_descendants_inclusive(root)
			.filter_map(|e| collections.get(e).ok())
			.filter(|(_, collection)| {
				collection.category.include_in_route_tree()
			})
			.map(|(e, _)| {
				children
					.iter_descendants(e)
					.filter_map(|c| methods.get(c).map(|m| (c, m)).ok())
			})
			.flatten()
			.collect::<Vec<_>>();

		let tree = RouteFileMethodTree::from_methods(child_methods);
		let parser = Parser { query: methods };
		codegen.add_item(parser.routes_mod_tree(&tree));
	}
}

/// Internal parser for building the route tree module structure.
#[derive(Clone)]
struct Parser<'w, 's, 'a> {
	/// Query for accessing route file methods.
	query: Query<'w, 's, &'a RouteFileMethod>,
}

impl<'a> Parser<'_, '_, 'a> {
	/// Gets a route file method by entity, panicking if not found.
	fn get(&self, entity: Entity) -> &RouteFileMethod {
		self.query.get(entity).expect(
			"Malformed RouteFileTree, entity does not have a RouteFileMethod component",
		)
	}

	/// Builds the top-level routes module from the tree.
	fn routes_mod_tree(&self, tree: &RouteFileMethodTree) -> Item {
		self.mod_tree_recursive(tree, true)
	}

	/// Recursively creates tree [`syn::Item`] nodes.
	///
	/// If a node has children, wraps them in a module of the same name.
	fn mod_tree_recursive(
		&self,
		tree: &RouteFileMethodTree,
		root: bool,
	) -> Item {
		let item = self.tree_path_func(tree);
		if !root && tree.children.is_empty() {
			item.map(|item| item.into())
				.unwrap_or(Item::Verbatim(TokenStream::default()))
		} else {
			let children = tree
				.children
				.iter()
				.map(|child| self.mod_tree_recursive(child, false));
			let ident = syn::Ident::new(
				&tree.name.to_string(),
				proc_macro2::Span::call_site(),
			);
			syn::parse_quote!(
				#[allow(unused, missing_docs)]
				pub mod #ident {
					use super::*;
					#item
					#(#children)*
				}
			)
		}
	}

	/// Generates a path function for a tree node.
	///
	/// For static paths, returns a function returning `&'static str`.
	/// For dynamic paths, returns a function with parameters for each dynamic segment.
	fn tree_path_func(&self, tree: &RouteFileMethodTree) -> Option<ItemFn> {
		// just use the first method, each func should have the same route path
		let Some(route) = &tree.funcs.iter().next() else {
			return None;
		};
		let route = self.get(**route);
		// if there are no children just use the name of the route,
		// otherwise specify `index` as the name
		let route_ident = if tree.children.is_empty() {
			let name = route
				.path
				.file_stem()
				.map(|s| s.to_string_lossy().to_snake_case())
				.unwrap_or("index".to_string());
			Ident::new(&name, Span::call_site())
		} else {
			Ident::new("index", Span::call_site())
		};
		let path_pattern = PathPattern::new(&route.path).unwrap();

		let func = if path_pattern.is_static() {
			let route_path = route.path.to_string_lossy().to_string();
			parse_quote!(
				pub fn #route_ident()-> &'static str{
					#route_path
				}
			)
		} else {
			let dyn_idents = path_pattern
				.iter()
				.filter_map(|segment| {
					if segment.is_static() {
						None
					} else {
						Some(Ident::new(segment.as_str(), Span::call_site()))
					}
				})
				.collect::<Vec<_>>();

			let raw_str = path_pattern
				.iter()
				.map(|segment| {
					if segment.is_static() {
						segment.as_str()
					} else {
						"{}"
					}
				})
				.collect::<Vec<_>>()
				.join("/");
			let raw_str = format!("/{raw_str}");

			parse_quote! {
				pub fn #route_ident(#(#dyn_idents: &str),*) -> String{
					format!(#raw_str, #(#dyn_idents),*)
				}
			}
		};
		Some(func)
	}
}


#[cfg(test)]
mod test {
	use super::Parser;
	use crate::prelude::*;
	use beet_core::prelude::*;
	use proc_macro2::TokenStream;
	use quote::ToTokens;

	fn world() -> World {
		let mut world = World::new();
		world.spawn_batch(vec![
			RouteFileMethod::new("/", HttpMethod::Get),
			RouteFileMethod::new("/bazz", HttpMethod::Get),
			RouteFileMethod::new("/foo/bar", HttpMethod::Get),
			RouteFileMethod::new("/foo/bazz", HttpMethod::Get),
			RouteFileMethod::new("/foo/bazz/boo", HttpMethod::Get),
			RouteFileMethod::new("/foo/bazz/boo", HttpMethod::Post),
		]);
		world
	}

	fn parse(methods: Vec<RouteFileMethod>) -> TokenStream {
		let mut world = World::new();
		world.spawn_batch(methods);
		let tree = RouteFileMethodTree::from_methods(
			world
				.query_once::<(Entity, &RouteFileMethod)>()
				.iter()
				.copied()
				.collect(),
		);
		let mut query = world.query::<&RouteFileMethod>();
		let query = query.query(&world);
		let mod_item = Parser { query }.routes_mod_tree(&tree);
		mod_item.to_token_stream()
	}

	#[test]
	fn single() {
		parse(vec![RouteFileMethod::new("/", HttpMethod::Get)])
			.xpect_snapshot();
	}

	#[test]
	fn dynamic() {
		parse(vec![RouteFileMethod::new(
			"/foo/:bar/*bazz",
			HttpMethod::Get,
		)])
		.xpect_snapshot();
	}

	#[test]
	fn empty() { parse(vec![]).xpect_snapshot(); }

	#[test]
	fn creates_mod() {
		let mut world = world();
		let methods = world
			.query_once::<(Entity, &RouteFileMethod)>()
			.iter()
			.copied()
			.collect();
		let tree = RouteFileMethodTree::from_methods(methods);
		let mut query = world.query::<&RouteFileMethod>();
		let query = query.query(&world);
		Parser { query }
			.routes_mod_tree(&tree)
			.to_token_stream()
			.xpect_snapshot();
	}
}
