//! The lowering-target runtime the `rsx!` macro emits into.
//!
//! `rsx!` lowers markup to a tree of [`Bundle`]s built on the
//! [`Element`](crate::prelude::Element)/[`Attribute`](crate::prelude::Attribute)/`children!`/[`Value`](crate::prelude::Value)
//! base, wrapped at the root by [`snippet`] into an
//! [`impl Template<Output = ()>`](Template) the substrate's `spawn_template`
//! accepts. This mirrors the marker-based blanket-impl pattern of
//! [`IntoBundle`](crate::prelude::IntoBundle), targeting the template substrate.
//!
//! Two traits carry the lowering:
//!
//! - [`IntoSnippet`] lifts a markup tree value (text, `{expr}`, `Vec`,
//!   `Option`, tuple) into a [`Bundle`]. This is what attribute values, text
//!   nodes, and child positions flow through.
//! - [`IntoSnippetBundle`] dispatches an uppercase tag or a bare-`{..}` spread:
//!   a [`Bundle`] (a reflect-patch component, a tuple, a helper) inserts onto
//!   the entity, a [`BuildTemplate`] builds its subtree into the entity. The
//!   macro cannot know which a path resolves to, so it lowers both through this
//!   trait.
use crate::prelude::*;
use alloc::sync::Arc;
use bevy::ecs::spawn::SpawnIter;
use bevy::ecs::system::IntoObserverSystem;
use bevy::ecs::template::Template;
use bevy::ecs::template::TemplateContext;
use bevy::platform::sync::Mutex;
use variadics_please::all_tuples;

/// A lowered `rsx!` tree (a *snippet*: a tree of XML nodes, the authored unit),
/// usable three ways:
///
/// - as an [`impl Template<Output = ()>`](Template) inserting its bundle into
///   the build target (`world.spawn_template(rsx!{..})`);
/// - as a [`Bundle`] (via [`BundleEffect`](crate::prelude::BundleEffect))
///   inserting its bundle directly (`world.spawn(rsx!{..})`,
///   `children![rsx!{..}]`, `(rsx!{..}, other)`);
/// - as an [`IntoSnippet`] value that, in a child position, builds into a fresh
///   child entity.
///
/// Holds the bundle as a take-once effect in a shared cell so it satisfies
/// [`Template::clone_template`] and `Clone` without the bundle being `Clone`.
#[derive(Clone, BundleEffect)]
pub struct Snippet(Arc<Mutex<Option<OnSpawn>>>);

impl Snippet {
	/// Wraps an `rsx!` markup bundle into a [`Snippet`]: the
	/// [`impl Template<Output = ()>`](Template) the substrate can spawn, and an
	/// [`IntoSnippet`] value usable in a child position.
	///
	/// `world.spawn_template(rsx!{ .. })` builds the whole tree into the spawned
	/// root; a `<div>{rsx!{..}}</div>` (or a helper returning `rsx!{..}`) builds
	/// the snippet into a fresh child entity. This is the single root adapter
	/// every `rsx!` invocation emits.
	///
	/// The bundle is owned, not cloned: `spawn_template` builds a template exactly
	/// once, so the inner effect is take-once. [`Template::clone_template`] is
	/// still satisfiable (it hands back the same shared slot), but a built snippet
	/// cannot be rebuilt, which matches the eager-build contract.
	pub fn from_bundle(bundle: impl Bundle) -> Self {
		Snippet(Arc::new(Mutex::new(Some(OnSpawn::insert(bundle)))))
	}

	/// Type-erase this snippet, for match arms whose branches build
	/// differently-shaped trees that `impl Trait` cannot unify.
	pub fn any_snippet(self) -> Snippet { self }

	/// Take and insert the held effect into `entity`, if not already built.
	fn build_into(&self, entity: &mut EntityWorldMut) {
		if let Some(effect) = self.0.lock().unwrap().take() {
			entity.insert(effect);
		}
	}

	/// The [`BundleEffect`](crate::prelude::BundleEffect) apply: insert the held
	/// bundle into the entity.
	fn effect(self, entity: &mut EntityWorldMut) { self.build_into(entity); }
}

impl Template for Snippet {
	type Output = ();
	fn build_template(&self, cx: &mut TemplateContext) -> Result<()> {
		self.build_into(cx.entity);
		OK
	}
	fn clone_template(&self) -> Self { self.clone() }
}

// `Snippet` is a `Bundle` (via `BundleEffect`), so it flows through the
// `IntoSnippet` bundle blanket: in a child position `children!` already spawns
// it as a fresh child entity, and its bundle is inserted there.

/// The storage type the `#[template]` derive uses for an optional or required
/// prop, decoupling the field type from `Option` so the `rsx!` call-site
/// conversion stays unambiguous.
///
/// A `#[template]` stores an `Option<T>`-declared or `#[prop(required)]` prop as
/// a `PropOpt<T>` and binds an `Option<T>` from it in the body. The call site
/// emits `field: value.into_prop()` ([`IntoProp`]); because `PropOpt<T>` has no
/// `From` impl, only the [`IntoProp`] `PropOpt` wrap applies (`placeholder="hi"`,
/// `variant=Variant::Error`), with no collision with `core`'s
/// `From<T> for Option<T>` that a bare `Option<T>` field would suffer.
#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Default)]
pub struct PropOpt<T>(pub Option<T>);

impl<T> Default for PropOpt<T> {
	fn default() -> Self { Self(None) }
}

impl<T> PropOpt<T> {
	/// Take the inner [`Option`], the form the `#[template]` body binds.
	pub fn into_inner(self) -> Option<T> { self.0 }

	/// Whether no value was supplied (used by the required-prop check).
	pub fn is_none(&self) -> bool { self.0.is_none() }
}

/// Convert a provided value into a `#[template]` prop field, the conversion the
/// `rsx!` component lowering uses for `<Foo field=value/>`.
///
/// A value flows directly into a non-`PropOpt` field via `From`, and into a
/// [`PropOpt<T>`] field by `Into<T>` then `Some`. The two never overlap because
/// `PropOpt` has no `From` impl, so inference resolves the marker `M` against the
/// field's known type.
pub trait IntoProp<F, M> {
	/// Convert into the field type `F`.
	fn into_prop(self) -> F;
}

/// Marker for the direct `From` conversion (a non-`PropOpt` field).
pub struct PropDirectMarker;
/// Marker for the [`PropOpt`] wrap conversion.
pub struct PropOptMarker;

/// A value flows directly into a field whose type is `From` it (the common case,
/// and the identity conversion when the field type matches).
impl<T, F: From<T>> IntoProp<F, PropDirectMarker> for T {
	fn into_prop(self) -> F { F::from(self) }
}

/// A value flows into a [`PropOpt<T>`] field by `Into<T>` then `Some`, so an
/// optional or required prop accepts the bare inner value.
impl<T, U: Into<T>> IntoProp<PropOpt<T>, PropOptMarker> for U {
	fn into_prop(self) -> PropOpt<T> { PropOpt(Some(self.into())) }
}

/// Lift `self` into a [`Bundle`] for an `rsx!` markup position (text, `{expr}`,
/// attribute value, child list). The marker `M` disambiguates the blanket impls,
/// mirroring [`IntoBundle`](crate::prelude::IntoBundle).
///
/// This is deliberately close to [`IntoBundle`]: the `rsx!` lowering routes
/// text and block child positions through [`IntoSnippet::into_snippet`] so
/// authors can write `<p>"Title: " {title}</p>` rather than spelling out
/// `Value::new(_)`.
pub trait IntoSnippet<M> {
	/// Lift into a bundle.
	fn into_snippet(self) -> impl Bundle;
}

/// Pass-through marker for values already a [`Bundle`].
pub struct SnippetBundleMarker;
/// All non-bundle impls carry this so they stay distinct from the bundle
/// pass-through in variadic tuples.
pub struct NotSnippetBundleMarker;
/// Marker for primitives lifted through `Into<Value>`.
pub struct SnippetValueMarker;
/// Marker distinguishing variadic tuple impls from the blanket impls.
pub struct SnippetTupleMarker;
/// Marker for the observer (`on*` handler) lift.
pub struct SnippetObserverMarker;

impl<T: Bundle> IntoSnippet<SnippetBundleMarker> for T {
	fn into_snippet(self) -> impl Bundle { self }
}

/// Primitives (`&str`, `String`, numerics, `bool`, …) become a
/// [`Value`](crate::prelude::Value).
impl<T: Into<Value>> IntoSnippet<(NotSnippetBundleMarker, SnippetValueMarker)>
	for T
{
	fn into_snippet(self) -> impl Bundle {
		let value: Value = self.into();
		value
	}
}

/// An `on*` event handler closure becomes an observer on the spawning entity.
impl<T, E, B, M>
	IntoSnippet<(NotSnippetBundleMarker, (SnippetObserverMarker, E, B, M))> for T
where
	E: Event,
	B: Bundle,
	T: 'static + Send + Sync + IntoObserverSystem<E, B, M>,
{
	fn into_snippet(self) -> impl Bundle { OnSpawn::observe(self) }
}

/// `Option<T>` — present items lift, absent ones are a no-op.
impl<T, M> IntoSnippet<(NotSnippetBundleMarker, (Option<T>, M))> for Option<T>
where
	T: IntoSnippet<M>,
{
	fn into_snippet(self) -> impl Bundle {
		match self {
			Some(item) => OnSpawn::insert(item.into_snippet()),
			None => OnSpawn::new(|_| {}),
		}
	}
}

/// Marker for the existing-[`Entity`] reparent lift.
pub struct SnippetEntityMarker;

/// An existing [`Entity`] in a child position becomes a child of the snippet, by
/// reference (it is reparented, not respawned). Mirrors
/// [`IntoBundle`](crate::prelude::IntoBundle) for [`Entity`].
impl IntoSnippet<(NotSnippetBundleMarker, SnippetEntityMarker)> for Entity {
	fn into_snippet(self) -> impl Bundle {
		OnSpawn::new(move |entity| {
			let id = entity.id();
			entity.world_scope(|world| {
				world.entity_mut(self).insert(ChildOf(id));
			});
		})
	}
}

/// `Vec<T>` — each item becomes its own child entity. Spawning each item as a
/// distinct child is required: tupling several single-component effects onto one
/// entity is last-write-wins and would silently drop all but the last.
impl<T, M> IntoSnippet<(NotSnippetBundleMarker, (Vec<T>, M))> for Vec<T>
where
	T: 'static + Send + Sync + IntoSnippet<M>,
{
	fn into_snippet(self) -> impl Bundle {
		Children::spawn(SpawnIter(
			self.into_iter().map(|item| item.into_snippet()),
		))
	}
}

macro_rules! impl_into_snippet_tuple {
	($(($T:ident, $t:ident, $M:ident)),*) => {
		impl<$($T, $M),*> IntoSnippet<(SnippetTupleMarker, ($($M,)*))> for ($($T,)*)
		where
			$($T: IntoSnippet<(NotSnippetBundleMarker, $M)>,)*
		{
			fn into_snippet(self) -> impl Bundle {
				let ($($t,)*) = self;
				($($t.into_snippet(),)*)
			}
		}
	}
}

all_tuples!(impl_into_snippet_tuple, 2, 15, T, t, M);

/// Marks a `#[template]` function-component data struct: a build-subtree
/// [`Template`](Template) that the `#[template]` derive implements.
///
/// This is the discriminator that lets [`IntoSnippetBundle`] dispatch a spread
/// or an uppercase tag between "insert as a component/bundle" and "build as a
/// template" without overlap. A reflect-patch [`Component`] does not implement
/// it (Bevy's blanket `Template for T: Default + Clone + Unpin` would otherwise
/// make a component look like a template); only a `#[template]` struct does,
/// because it opts out of `Unpin` via `subtree_template!` and supplies its own
/// build.
pub trait BuildTemplate:
	'static + Send + Sync + Clone + Template<Output = ()>
{
}

/// Dispatch an uppercase tag or a bare-`{..}` spread into a [`Bundle`].
///
/// The `rsx!` macro cannot know whether `<Foo a=x/>` resolves to a
/// [`Component`] or a build-subtree [`Template`](Template), so it lowers both as
/// `Foo { a: x.into(), ..Default::default() }.into_snippet_bundle()` and
/// dispatches here:
///
/// - a [`Bundle`] (a patched-over-default reflect [`Component`], a tuple, or a
///   helper like [`attr`](crate::prelude::attr)) is inserted onto the entity;
/// - a [`BuildTemplate`] (a `#[template]` function component) builds its subtree
///   into the entity, carrying its
///   [`SlotTarget`](crate::prelude::SlotTarget) markers for the walker.
///
/// `Option<T>` composes for free, so `{field}` with `field: Option<FieldRef>`
/// works.
pub trait IntoSnippetBundle<M> {
	/// Lift into a bundle that inserts the component/bundle or builds the
	/// template.
	fn into_snippet_bundle(self) -> impl Bundle;
}

/// Marker for the [`Bundle`] (component/tuple/helper) insert dispatch.
pub struct SnippetBundleSpreadMarker;
/// Marker for the build-subtree [`BuildTemplate`] dispatch.
pub struct SnippetBuildTemplateMarker;
/// Marker for the [`Option`] spread dispatch.
pub struct SnippetOptionSpreadMarker;

/// A [`Bundle`] tag/spread inserts itself onto the entity (last-write-wins, so a
/// duplicate component compose is the patch the author intends).
impl<B: Bundle> IntoSnippetBundle<SnippetBundleSpreadMarker> for B {
	fn into_snippet_bundle(self) -> impl Bundle { self }
}

/// A build-subtree template builds into the entity via the substrate's
/// `build_template`, without firing the root lifecycle: the surrounding
/// `spawn_template` owns slot resolution and lifecycle across the whole tree.
impl<T: BuildTemplate> IntoSnippetBundle<SnippetBuildTemplateMarker> for T {
	fn into_snippet_bundle(self) -> impl Bundle {
		OnSpawnClone::new(move |entity| {
			let template = self.clone();
			// build the template's subtree into this entity; a build failure rides
			// `TemplateError` so it surfaces through the root's `LoadTemplate`.
			if let Err(error) = entity.build_template(&template) {
				entity.insert(TemplateError::new(error));
			}
		})
	}
}

/// `Option<T>` spreads its inner component/template when present, nothing when
/// absent. This is what makes `{field}` work for `field: Option<FieldRef>`.
impl<T, M> IntoSnippetBundle<(SnippetOptionSpreadMarker, M)> for Option<T>
where
	T: 'static + Send + Sync + IntoSnippetBundle<M>,
{
	fn into_snippet_bundle(self) -> impl Bundle {
		OnSpawn::insert_option(self.map(|item| item.into_snippet_bundle()))
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	fn is_snippet<M>(_: impl IntoSnippet<M>) {}

	#[beet_core::test]
	fn lifts_markup_values() {
		// pass-through bundle
		is_snippet(Name::new("hi"));
		// primitives via Into<Value>
		is_snippet("hello");
		is_snippet(String::from("hello"));
		is_snippet(42_i32);
		is_snippet(true);
		// children list
		is_snippet(vec!["a", "b", "c"]);
		// tuple
		is_snippet((1_i32, "two"));
		// optional component
		is_snippet(Some(Name::new("name")));
		is_snippet(None::<Name>);
	}
}
