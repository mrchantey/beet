//! The components a document declares at its ROOT, read without building it.

use super::ast::*;
use super::reflect::*;
use super::resolve::insert_component;
use crate::prelude::*;
use bevy::ecs::reflect::ReflectComponent;
use bevy::reflect::PartialReflect;
use bevy::reflect::TypeRegistry;
use bevy::reflect::Typed;
use core::any::TypeId;

/// The components a document declares at its ROOT, as literals, before any
/// registry has seen them.
///
/// The authoring surfaces name the same thing two ways — a BSX page writes
/// bare-position spreads on its root element
/// (`<Fragment {PageMeta{title:".."}}>`), markdown writes frontmatter
/// (`+++ title = ".." +++`) — and both lower to this. So a document's root
/// metadata is *whatever components it declares*, never one blessed type:
/// splitting a metadata component in two, or a site declaring its own, needs no
/// change here.
///
/// A scan reads the root WITHOUT building the document: nothing is spawned, no
/// component hook runs, no child is built and no route is registered. That is
/// what lets route discovery know a page's title, order and slug before anyone
/// visits it. Only the roots, and only spreads: a nested element's spread is
/// page content, belonging to the page rather than to the route serving it.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct RootDeclarations(pub Vec<NamedLiteral>);

impl RootDeclarations {
	/// The declarations a parsed BSX document names in bare-position spreads on
	/// its root elements, in author order.
	pub fn from_bsx(nodes: &[BsxNode]) -> Self {
		nodes
			.iter()
			.filter_map(|node| match node {
				BsxNode::Element(el) => Some(el),
				_ => None,
			})
			.flat_map(|el| el.attributes.iter())
			.filter_map(|attr| match &attr.value {
				AttrValue::Spread(spread) => Some(spread),
				_ => None,
			})
			.flat_map(|spread| match spread {
				SpreadExpr::Named(named) => vec![named.clone()],
				SpreadExpr::Tuple(items) => items
					.iter()
					.filter_map(|item| match item {
						SpreadItem::Named(named) => Some(named.clone()),
						SpreadItem::Binding(_) => None,
					})
					.collect(),
			})
			.collect::<Vec<_>>()
			.xmap(Self)
	}

	/// Whether the document declared nothing at its root.
	pub fn is_empty(&self) -> bool { self.0.is_empty() }

	/// Reflect-build the declaration naming `T`, patched over `T::default()`, for
	/// a consumer wanting one specific type (the router reading a page's `slug`
	/// before the route entity exists).
	///
	/// `None` when no declaration names `T`, when `T` is absent from `registry`,
	/// or when the literal does not fit it.
	pub fn get<T: Default + Reflect + Typed>(
		&self,
		registry: &TypeRegistry,
	) -> Option<T> {
		let name = T::type_info().type_path_table().short_path();
		let named = self.0.iter().find(|named| named.name == name)?;
		let patch =
			to_patch(named, registry.get(TypeId::of::<T>())?, registry).ok()?;
		let mut value = T::default();
		value.try_apply(patch.as_ref()).ok()?;
		Some(value)
	}

	/// Declare `field` on the `component` declaration when the document named
	/// that component WITHOUT it: a default something other than the document
	/// implies, ie a content file's numbered filename.
	///
	/// Never creates a declaration — a document naming no such component declared
	/// no such metadata, and a filename does not change that — and never touches
	/// a tuple declaration, which has no named field to fill.
	pub fn declare_default(
		&mut self,
		component: &str,
		field: &str,
		value: DataLiteral,
	) {
		let Some(named) =
			self.0.iter_mut().find(|named| named.name == component)
		else {
			return;
		};
		match &mut named.fields {
			NamedFields::Struct(fields) => {
				if !fields.iter().any(|(key, _)| key == field) {
					fields.push((field.into(), value));
				}
			}
			NamedFields::Unit => {
				named.fields = NamedFields::Struct(vec![(field.into(), value)]);
			}
			NamedFields::Tuple(_) => {}
		}
	}

	/// Insert every component these declarations name onto `entity`, reflect-built
	/// against the world's type registry and patched over each type's default.
	///
	/// A declaration naming a registered *template* or *resource* is skipped: those
	/// belong to the document's build, not to its metadata. A name nothing
	/// registers warns and is skipped, so a lean binary hoists the subset it links
	/// rather than failing the whole load.
	pub fn insert(&self, entity: &mut EntityWorldMut) -> Result<()> {
		if self.is_empty() {
			return Ok(());
		}
		let app_registry = entity
			.world_scope(|world| {
				world.get_resource::<AppTypeRegistry>().cloned()
			})
			.ok_or_else(|| {
				bevyhow!(
					"root declarations require an `AppTypeRegistry` in the world"
				)
			})?;
		for named in &self.0 {
			let patch = {
				let registry = app_registry.read();
				let Some(registration) =
					registration_by_name(&registry, &named.name)
				else {
					warn!(
						"skipping root declaration `{}`: no component of that name is registered in this binary",
						named.name
					);
					continue;
				};
				if registration.data::<ReflectComponent>().is_none() {
					continue;
				}
				to_patch(named, registration, &registry)?
			};
			insert_component(entity, patch.as_ref(), &app_registry)?;
		}
		Ok(())
	}

	/// A bundle inserting every declared component as the entity spawns, for a
	/// caller composing a route rather than holding the world: the codegen-emitted
	/// twin of the discovery scan's [`insert`](Self::insert).
	pub fn hoist(self) -> impl Bundle {
		OnSpawn::new(move |entity: &mut EntityWorldMut| self.insert(entity))
	}
}

/// Reflect-build one declaration against its registration, the shared step every
/// surface resolves through: the string coercions (`YYYY-MM-DD` to [`Timestamp`],
/// `SmolStr`, `Option`, enum variants) come from [`DataLiteral::to_reflect`].
fn to_patch(
	named: &NamedLiteral,
	registration: &bevy::reflect::TypeRegistration,
	registry: &TypeRegistry,
) -> Result<Box<dyn PartialReflect>> {
	// a root declaration carries no `$ref`, having no tree to resolve one against
	let mut resolver = |_: &str| Entity::PLACEHOLDER;
	DataLiteral::to_reflect(
		&DataLiteral::Enum(named.clone()),
		Some(registration.type_info()),
		registry,
		&mut resolver,
	)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// A nested struct field, so the scan covers a declaration that is not flat.
	#[derive(Debug, Default, Clone, PartialEq, Reflect)]
	struct Nested {
		label: Option<String>,
		order: Option<u32>,
	}

	/// A page-metadata shaped component, the scan-time declaration `RoutesDir`
	/// reads.
	#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
	#[reflect(Component, Default)]
	struct Meta {
		title: Option<String>,
		draft: bool,
		order: Option<u32>,
		nested: Nested,
		created: Option<Timestamp>,
	}

	/// A second root component, proving the scan hoists whatever a document
	/// declares rather than one blessed type.
	#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
	#[reflect(Component, Default)]
	struct Extra {
		note: Option<String>,
	}

	fn scan_world() -> World {
		let mut world = World::new();
		world.init_resource::<AppTypeRegistry>();
		let mut registry = world.resource_mut::<AppTypeRegistry>();
		registry.write().register::<Meta>();
		registry.write().register::<Extra>();
		world
	}

	fn scan(markup: &str) -> RootDeclarations {
		BsxNode::parse_document(markup, &BsxParseConfig::bsx())
			.unwrap()
			.xmap(|nodes| RootDeclarations::from_bsx(&nodes))
	}

	/// A root spread reads without building: the document is never spawned, so
	/// nothing but the named components is produced.
	#[crate::test]
	fn scans_root_spread() {
		let world = scan_world();
		let app_registry = world.resource::<AppTypeRegistry>();
		let registry = app_registry.read();
		let get = |markup: &str| scan(markup).get::<Meta>(&registry);
		// the declared fields land, the rest fill from `Default`
		get(r#"<Fragment {Meta{title:"Blog", nested: Nested{order: 1}}}><h1>hi</h1></Fragment>"#)
			.unwrap()
			.xpect_eq(Meta {
				title: Some("Blog".into()),
				draft: false,
				order: None,
				nested: Nested {
					label: None,
					order: Some(1),
				},
				created: None,
			});
		// a `YYYY-MM-DD` string coerces to the instant it names, through the
		// `Option` wrapper the field declares
		get(r#"<Fragment {Meta{created:"2025-09-06"}}/>"#)
			.unwrap()
			.created
			.unwrap()
			.format_date()
			.xpect_eq("2025-09-06");
		// a tuple spread names it alongside others
		get(r#"<Fragment {(Meta{draft: true}, PackageConfig)}/>"#)
			.unwrap()
			.draft
			.xpect_true();
		// only the ROOT declares route metadata; a nested spread is page content
		get(r#"<Fragment><div {Meta{title:"nested"}}/></Fragment>"#)
			.is_none()
			.xpect_true();
	}

	/// Every root declaration hoists, not just the one a caller asked for.
	#[crate::test]
	fn hoists_every_declaration() {
		let mut world = scan_world();
		let declarations =
			scan(r#"<Fragment {(Meta{title:"Blog"}, Extra{note:"hi"})}/>"#);
		let entity = world
			.spawn_empty()
			.xtap(|entity| declarations.insert(entity).unwrap())
			.id();
		world
			.entity(entity)
			.get::<Meta>()
			.unwrap()
			.title
			.as_deref()
			.unwrap()
			.xpect_eq("Blog");
		world
			.entity(entity)
			.get::<Extra>()
			.unwrap()
			.note
			.as_deref()
			.unwrap()
			.xpect_eq("hi");
	}

	/// A filename-implied default fills a field the document left out, and never
	/// invents a declaration the document never made.
	#[crate::test]
	fn declare_default_fills_undeclared_fields() {
		let world = scan_world();
		let app_registry = world.resource::<AppTypeRegistry>();
		let registry = app_registry.read();
		let mut declarations = scan(r#"<Fragment {Meta{title:"Blog"}}/>"#);
		declarations.declare_default(
			"Meta",
			"order",
			DataLiteral::Scalar(Value::Uint(3)),
		);
		// an explicitly declared field wins over the default
		declarations.declare_default(
			"Meta",
			"title",
			DataLiteral::Scalar(Value::str("other")),
		);
		let meta = declarations.get::<Meta>(&registry).unwrap();
		meta.order.unwrap().xpect_eq(3);
		meta.title.as_deref().unwrap().xpect_eq("Blog");
		// an undeclared component stays undeclared
		let mut none = RootDeclarations::default();
		none.declare_default(
			"Meta",
			"order",
			DataLiteral::Scalar(Value::Uint(3)),
		);
		none.get::<Meta>(&registry).is_none().xpect_true();
	}
}
