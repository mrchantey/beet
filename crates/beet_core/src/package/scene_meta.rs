//! [`SceneMeta`] and [`PackageMeta`]: what a component contributes to the
//! record beside a compiled scene, and to its package record.
use crate::prelude::*;
use alloc::collections::BTreeMap;
use bevy::ecs::component::Mutable;
use bevy::reflect::CreateTypeData;

/// A component taking part in compilation, registered as type data with
/// `#[reflect(SceneMeta)]` so `app.register_type` is the whole registration.
///
/// The compiler spawns a document's skeleton in a compile world to which no
/// plugin was added: one entity per body entity with its hierarchy, carrying
/// only the components with this data and their requirements, so no hook of
/// any other component runs at compile. It then spawns the meta entity, which
/// becomes the scene's record, and calls `apply` for every component with this
/// data, in body order.
///
/// An `apply` reads the skeleton and the body, writes only its own entity and
/// the meta entity, never spawns, despawns or reparents, and never depends on
/// another `apply` having run, so the result is independent of plugin order:
/// several writing one meta component accumulate into it, a union whatever
/// the order ([`SceneMetaContext::meta_mut`]). A meta component's own hooks and
/// requirements do run, so its author keeps them compile safe.
///
/// Scene meta sees one document: an [`Instance`] is a reference here, never
/// expanded, so the meta is a function of the document alone. What needs other
/// documents is [`PackageMeta`].
///
/// ```
/// # use beet_core::prelude::*;
/// #[derive(Default, Component, Reflect)]
/// #[reflect(Component, SceneMeta)]
/// struct Title(String);
///
/// impl SceneMeta for Title {
/// 	fn apply(cx: &mut SceneMetaContext) -> Result {
/// 		let title = cx.get::<Self>()?.0.clone();
/// 		cx.insert_meta(Title(title));
/// 		Ok(())
/// 	}
/// }
/// ```
pub trait SceneMeta {
	/// Contribute to the scene's meta entity.
	fn apply(cx: &mut SceneMetaContext) -> Result;
}

/// [`SceneMeta`] as type data.
#[derive(Debug, Clone)]
pub struct ReflectSceneMeta {
	apply: fn(&mut SceneMetaContext) -> Result,
}

impl ReflectSceneMeta {
	/// Run the type's [`SceneMeta::apply`].
	pub fn apply(&self, cx: &mut SceneMetaContext) -> Result {
		(self.apply)(cx)
	}
}

impl<T: SceneMeta> CreateTypeData<T> for ReflectSceneMeta {
	fn create_type_data(_input: ()) -> Self { Self { apply: T::apply } }
}

/// The compile world's view for one [`SceneMeta::apply`]: the entity carrying
/// the component, its key, the whole lowered body as data and the meta entity
/// being built. Symmetrical, since most applies read one and write the other.
#[derive(Deref, DerefMut)]
pub struct SceneMetaContext<'a> {
	/// The compile world, holding the skeleton.
	#[deref]
	pub world: &'a mut World,
	/// Every component of every entity as data; the skeleton holds only the
	/// meta components.
	pub body: &'a SceneBody,
	/// The document's path in its package.
	pub document: &'a RelPath,
	/// The skeleton entity carrying the component.
	pub entity: Entity,
	/// Its key in the body.
	pub key: &'a EntityKey,
	/// The meta entity, which becomes the record.
	pub meta: Entity,
}

impl SceneMetaContext<'_> {
	/// The `T` on the entity being applied, ie the component itself.
	pub fn get<T: Component>(&self) -> Result<&T> {
		self.world.entity(self.entity).get::<T>().ok_or_else(|| {
			bevyhow!(
				"`{}` holds no `{}` at `{}`",
				self.document,
				ShortName::of::<T>(),
				self.key
			)
		})
	}

	/// The skeleton parent of the entity being applied.
	pub fn parent(&self) -> Option<Entity> {
		self.world
			.entity(self.entity)
			.get::<ChildOf>()
			.map(ChildOf::parent)
	}

	/// Whether the entity being applied is the document's root.
	pub fn is_root(&self) -> bool { self.parent().is_none() }

	/// The `T` on the meta entity, inserted by default where absent: what an
	/// accumulating apply writes into.
	pub fn meta_mut<T: Component<Mutability = Mutable> + Default>(
		&mut self,
	) -> Mut<'_, T> {
		let meta = self.meta;
		let mut entity = self.world.entity_mut(meta);
		if !entity.contains::<T>() {
			entity.insert(T::default());
		}
		self.world.get_mut::<T>(meta).expect("inserted just above")
	}

	/// Insert `bundle` on the meta entity.
	pub fn insert_meta(&mut self, bundle: impl Bundle) {
		let meta = self.meta;
		self.world.entity_mut(meta).insert(bundle);
	}

	/// An error naming where the component being applied was written.
	pub fn error(&self, message: impl core::fmt::Display) -> BevyError {
		bevyhow!("`{}` at `{}`: {message}", self.document, self.key)
	}
}

/// A component taking part in the package record, registered with
/// `#[reflect(PackageMeta)]`: run once the whole branch is compiled, for every
/// scene record carrying it, so it may see beyond one document (every file,
/// every scene's meta), where [`SceneMeta`] may not.
///
/// The component reaches here as scene meta: its [`SceneMeta::apply`] hoisted
/// it onto a scene record, and this reads it back off that record
/// ([`PackageMetaContext::scene`]) and writes the package's meta entity, under
/// the same rules as scene meta.
pub trait PackageMeta {
	/// Contribute to the package record's meta entity.
	fn apply(cx: &mut PackageMetaContext) -> Result;
}

/// [`PackageMeta`] as type data.
#[derive(Debug, Clone)]
pub struct ReflectPackageMeta {
	apply: fn(&mut PackageMetaContext) -> Result,
}

impl ReflectPackageMeta {
	/// Run the type's [`PackageMeta::apply`].
	pub fn apply(&self, cx: &mut PackageMetaContext) -> Result {
		(self.apply)(cx)
	}
}

impl<T: PackageMeta> CreateTypeData<T> for ReflectPackageMeta {
	fn create_type_data(_input: ()) -> Self { Self { apply: T::apply } }
}

/// The compile world's view for one [`PackageMeta::apply`]: the branch's files,
/// the record of every scene in reach by its path, the document whose scene
/// carries the component, and the package's meta entity.
#[derive(Deref, DerefMut)]
pub struct PackageMetaContext<'a> {
	/// The compile world, holding the package's meta entity.
	#[deref]
	pub world: &'a mut World,
	/// Every file of the branch.
	pub files: &'a Files,
	/// Every scene record in reach, by its document's path.
	pub scenes: &'a BTreeMap<RelPath, EntityBodyRecord>,
	/// The document whose scene carries the component.
	pub document: &'a RelPath,
	/// The package's meta entity, which becomes the package record.
	pub meta: Entity,
}

impl PackageMetaContext<'_> {
	/// The record of the scene carrying the component.
	pub fn scene(&self) -> &EntityBodyRecord {
		self.scenes
			.get(self.document)
			.expect("the compiler applies a scene in reach")
	}

	/// The `T` on the package's meta entity, inserted by default where absent.
	pub fn meta_mut<T: Component<Mutability = Mutable> + Default>(
		&mut self,
	) -> Mut<'_, T> {
		let meta = self.meta;
		let mut entity = self.world.entity_mut(meta);
		if !entity.contains::<T>() {
			entity.insert(T::default());
		}
		self.world.get_mut::<T>(meta).expect("inserted just above")
	}
}
