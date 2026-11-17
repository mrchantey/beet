//! Templates held unbuilt: content a component carries and builds later, as
//! many times as it likes.
//!
//! A tag's children normally build where the tag sits. A component that
//! decides when its content builds (a server island resolving per request, a
//! fallback shown only in one render) holds it as a [`HeldTemplate`]
//! instead. In markup it implements [`HoldChildren`] and registers
//! `#[reflect(HoldChildren)]`, and the BSX build hands it its children, grouped
//! by the slot each names, rather than building them; in `rsx!` the content is
//! a field, `content={|| rsx!{ .. }}`.
use crate::prelude::*;
use alloc::sync::Arc;
use bevy::ecs::component::Mutable;
use bevy::ecs::template::Template;
use bevy::reflect::CreateTypeData;

/// The erased build a [`HeldTemplate`] holds.
type BuildFn = Arc<dyn Fn(&mut EntityWorldMut) -> Result + Send + Sync>;

/// A template held unbuilt, built into an entity on demand as many times as its
/// holder likes. The default holds nothing and builds nothing.
///
/// ```
/// # use beet_core::prelude::*;
/// let content = HeldTemplate::new(|| Snippet::from_bundle(Element::new("p")));
/// let mut world = World::new();
/// let entity = world.spawn_empty().id();
/// content.build(&mut world.entity_mut(entity)).unwrap();
/// world.entity(entity).contains::<Element>().xpect_true();
/// ```
#[derive(Clone, Default)]
pub struct HeldTemplate(Option<BuildFn>);

impl HeldTemplate {
	/// Build what `func` returns each time this builds, ie
	/// `HeldTemplate::new(|| rsx!{ <VisitCount/> })`.
	pub fn new<T: Template<Output = ()>>(
		func: impl 'static + Send + Sync + Fn() -> T,
	) -> Self {
		Self(Some(Arc::new(move |entity: &mut EntityWorldMut| {
			entity.insert_template(func()).map(|_| ())
		})))
	}

	/// Build a clone of `template` each time this builds, ie a parsed BSX
	/// subtree.
	pub fn from_template<T>(template: T) -> Self
	where
		T: 'static + Send + Sync + Clone + Template<Output = ()>,
	{
		Self::new(move || template.clone())
	}

	/// Whether this holds nothing, so building it builds nothing.
	pub fn is_empty(&self) -> bool { self.0.is_none() }

	/// Build the held template into `entity`, which becomes its template root,
	/// so a deferred dependency it parks lands on `entity`'s
	/// [`TemplatePending`]. Holding nothing builds nothing.
	pub fn build(&self, entity: &mut EntityWorldMut) -> Result {
		match &self.0 {
			Some(func) => func(entity),
			None => Ok(()),
		}
	}
}

impl core::fmt::Debug for HeldTemplate {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self.0 {
			Some(_) => f.write_str("HeldTemplate(..)"),
			None => f.write_str("HeldTemplate(empty)"),
		}
	}
}

/// An `rsx!` prop takes a closure where its field is a [`HeldTemplate`],
/// ie `<ServerIsland name="visits" content={|| rsx!{ <VisitCount/> }}/>`.
impl<F, T> IntoProp<HeldTemplate> for F
where
	F: 'static + Send + Sync + Fn() -> T,
	T: Template<Output = ()>,
{
	fn into_prop(self) -> HeldTemplate { HeldTemplate::new(self) }
}

/// Children a tag holds unbuilt, grouped by the slot each names: the unrouted
/// ones in the default group, a `slot="fallback"` child in the `fallback`
/// group.
#[derive(Debug, Clone, Default)]
pub struct HeldChildren {
	default: HeldTemplate,
	named: HashMap<SmolStr, HeldTemplate>,
}

impl HeldChildren {
	/// Children held as `default`, the unrouted group, and `named`, one group
	/// per slot name.
	pub fn new(
		default: HeldTemplate,
		named: HashMap<SmolStr, HeldTemplate>,
	) -> Self {
		Self { default, named }
	}

	/// Take the unrouted children.
	pub fn take_default(&mut self) -> HeldTemplate {
		core::mem::take(&mut self.default)
	}

	/// Take the children routed to slot `name`, empty when none were.
	pub fn take(&mut self, name: &str) -> HeldTemplate {
		self.named.remove(name).unwrap_or_default()
	}

	/// The slot names no [`take`](Self::take) claimed, ie a typo'd
	/// `slot="fallbak"` a holder refuses.
	pub fn unclaimed(&self) -> impl Iterator<Item = &SmolStr> {
		self.named.keys()
	}
}

/// A component holding its authored children unbuilt rather than building
/// them where it sits. Register `#[reflect(HoldChildren)]` and the BSX build
/// hands it its children as [`HeldChildren`] after inserting it.
pub trait HoldChildren {
	/// Take the authored `children`, refusing any group it has no use for.
	fn hold_children(&mut self, children: HeldChildren) -> Result;
}

/// Reflect type data for a [`HoldChildren`] component, so the BSX build hands
/// children to a tag known only by name.
#[derive(Clone)]
pub struct ReflectHoldChildren {
	hold: fn(&mut EntityWorldMut, HeldChildren) -> Result,
}

impl ReflectHoldChildren {
	/// Hand `children` to the component on `entity`.
	pub fn hold(
		&self,
		entity: &mut EntityWorldMut,
		children: HeldChildren,
	) -> Result {
		(self.hold)(entity, children)
	}
}

impl<T> CreateTypeData<T> for ReflectHoldChildren
where
	T: Component<Mutability = Mutable> + HoldChildren,
{
	fn create_type_data(_input: ()) -> Self {
		Self {
			hold: |entity, children| {
				entity
					.get_mut::<T>()
					.ok_or_else(|| {
						bevyhow!(
							"`{}` holds children but is not on the entity",
							core::any::type_name::<T>()
						)
					})?
					.hold_children(children)
			},
		}
	}
}
