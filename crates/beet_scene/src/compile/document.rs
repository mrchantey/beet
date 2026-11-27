//! One document compiled into a scene: its references resolved, its skeleton
//! spawned in a compile world, and its scene meta hoisted onto the record.
use super::*;
use crate::prelude::*;
use alloc::collections::BTreeMap;
use beet_core::prelude::*;
use bevy::ecs::reflect::ReflectComponent;
use bevy_reflect::TypeRegistry;
use bevy_reflect::serde::TypedReflectSerializer;
use core::any::TypeId;

/// One document compiled: the scene body with every reference package
/// absolute and complete, its postcard bytes, the record they key, and what
/// package meta checks of it.
pub(super) struct CompiledDocument {
	/// The document's path in its package.
	pub path: RelPath,
	/// The document as it was read, for an error's position.
	pub source: DocumentSource,
	/// The scene body's blob, postcard bytes of the resolved body.
	pub blob: Vec<u8>,
	/// The record over that blob, scene meta and all. Its provenance is set
	/// as it is written.
	pub record: EntityBodyRecord,
	/// Every [`Instance`] the document makes, for the props check.
	pub uses: Vec<InstanceUse>,
}

impl CompiledDocument {
	/// The reference to the body's blob, which keys the record.
	pub fn blob_ref(&self) -> BlobRef {
		self.record
			.get::<BlobRef>()
			.expect("a compiled document's record holds its blob")
	}

	/// Where the entity at `key` was written, its document's path where it
	/// has none.
	pub fn position(&self, key: &EntityKey) -> String {
		self.source
			.position(key)
			.map(|position| position.to_string())
			.unwrap_or_else(|| format!("{} at `{key}`", self.path))
	}
}

/// One use of another document, as its props are checked.
pub(super) struct InstanceUse {
	/// The use site.
	pub key: EntityKey,
	/// The target, package absolute and complete.
	pub document: RelPath,
	/// The props the use passes.
	pub props: Map,
}

/// Every name a document's references may take: this package's files, and
/// each dependency's under the prefix documents name it by.
#[derive(Default)]
pub(super) struct Namespace {
	pub files: Files,
	pub dependencies: BTreeMap<PackageName, Files>,
}

impl Namespace {
	/// The file `reference`, written in the document at `document`, names:
	/// package absolute with its extension, through a dependency's prefix
	/// where its first segment is one.
	///
	/// ## Errors
	/// Errors when it names no file, or names two by leaving off an
	/// extension they differ by.
	pub fn complete(
		&self,
		document: &RelPath,
		reference: &DocPath,
	) -> Result<RelPath> {
		let path = reference.resolve(document);
		let (prefix, files, within) =
			match path.as_str().split_once('/').and_then(|(first, rest)| {
				PackageName::parse(first)
					.ok()
					.and_then(|key| self.dependencies.get_key_value(&key))
					.map(|(key, files)| (Some(key), files, RelPath::new(rest)))
			}) {
				Some(found) => found,
				None => (None, &self.files, path.clone()),
			};
		let prefixed = |path: RelPath| match prefix {
			Some(prefix) => RelPath::new(prefix.as_str()).join(path.as_str()),
			None => path,
		};
		match files.completions(&within).as_slice() {
			[] => {
				bevybail!("`{reference}` names no file of the {}", match prefix
				{
					Some(prefix) => format!("dependency `{prefix}`"),
					None => "package".into(),
				})
			}
			[found] => prefixed(found.clone()).xok(),
			many => bevybail!(
				"`{reference}` is ambiguous, naming {}: write its extension",
				many.iter()
					.map(|path| format!("`{}`", prefixed(path.clone())))
					.collect::<Vec<_>>()
					.join(" and ")
			),
		}
	}
}

impl CompiledDocument {
	/// Compile the document at `path` from `bytes`: lower it, resolve every
	/// reference against `namespace`, then hoist its scene meta in a compile
	/// world whose registry is `parsers`' types and to which no plugin was
	/// added.
	pub fn compile(
		parsers: &DocParsers,
		namespace: &Namespace,
		path: &RelPath,
		bytes: &MediaBytes,
	) -> Result<Self> {
		let source = parsers.read(bytes, Some(path))?;
		let mut body = source.body().clone();
		let types = parsers.types().clone();
		let registry = types.read();
		let located =
			|key: &EntityKey, err: BevyError| match source.position(key) {
				Some(position) => bevyhow!("{position}: {err}"),
				None => bevyhow!("{path} at `{key}`: {err}"),
			};

		// resolve every reference, recording each edge and every use
		let mut imports = Imports::default();
		let mut uses = Vec::new();
		let keys = EntityKeys::new(body.entities.keys().cloned());
		for (key, entity) in body.entities.iter_mut() {
			Self::resolve_entity(
				&registry,
				&keys,
				namespace,
				path,
				key,
				entity,
				&mut imports,
				&mut uses,
			)
			.map_err(|err| located(key, err))?;
		}

		// the meta entity, hoisted from the skeleton
		let mut record = Self::scene_meta(&types, &registry, &body, path)
			.map_err(|err| {
				match err
					.downcast_ref::<MetaError>()
					.map(|meta| meta.key.clone())
				{
					Some(key) => located(&key, err),
					None => err,
				}
			})?;
		if !imports.0.is_empty() {
			record.insert(&imports)?;
		}
		let blob = MediaType::Postcard.serialize(&body)?;
		record.insert(&BlobRef::of(&blob, MediaType::Postcard))?;
		Self {
			path: path.clone(),
			source,
			blob,
			record,
			uses,
		}
		.xok()
	}

	/// The scene meta of the document at `path` as written, nothing resolved:
	/// what an entry declares (its [`Package`] and dependencies), read before
	/// any reference can resolve.
	pub fn declarations(
		parsers: &DocParsers,
		path: &RelPath,
		bytes: &MediaBytes,
	) -> Result<EntityBodyRecord> {
		let source = parsers.read(bytes, Some(path))?;
		let types = parsers.types().clone();
		let registry = types.read();
		Self::scene_meta(&types, &registry, source.body(), path).map_err(
			|err| match err
				.downcast_ref::<MetaError>()
				.map(|meta| meta.key.clone())
			{
				Some(key) => match source.position(&key) {
					Some(position) => bevyhow!("{position}: {err}"),
					None => bevyhow!("{path} at `{key}`: {err}"),
				},
				None => err,
			},
		)
	}

	/// Resolve every reference one entity's components hold, rewriting each
	/// where it was written.
	#[allow(clippy::too_many_arguments)]
	fn resolve_entity(
		registry: &TypeRegistry,
		keys: &EntityKeys,
		namespace: &Namespace,
		path: &RelPath,
		key: &EntityKey,
		entity: &mut EntityBody,
		imports: &mut Imports,
		uses: &mut Vec<InstanceUse>,
	) -> Result {
		let kind = match (
			Self::holds::<LoadRoot>(registry, entity),
			Self::holds::<Preload>(registry, entity),
		) {
			(true, true) => ImportKind::Preload,
			(true, false) => ImportKind::Lazy,
			_ => ImportKind::Eager,
		};
		for (name, value) in entity.iter_mut() {
			let Some(registration) =
				DynamicTemplate::registration(registry, name)
			else {
				continue;
			};
			let instance = registration.type_id() == TypeId::of::<Instance>();
			let reflected = keys.deserialize(registration, registry, value)?;
			let mut found = |reference: &DocPath, text: Option<&mut Value>| {
				let resolved = namespace
					.complete(path, reference)
					.map_err(|err| bevyhow!("`{name}`: {err}"))?;
				if let Some(text) = text {
					*text = Value::Str(resolved.as_str().into());
				}
				imports.insert(resolved, ImportEdge {
					kind: match instance {
						true => kind,
						false => ImportKind::Eager,
					},
					instance,
				});
				Ok(())
			};
			ReferenceWalk::new(&mut found)
				.walk(reflected.as_ref(), Some(value))?;
			if instance
				&& let Some(use_site) = reflected
					.try_as_reflect()
					.and_then(|reflected| reflected.downcast_ref::<Instance>())
			{
				uses.push(InstanceUse {
					key: key.clone(),
					document: namespace.complete(path, &use_site.document)?,
					props: use_site.props.clone(),
				});
			}
		}
		Ok(())
	}

	/// Whether `entity` holds a `T`, by any name a body writes it under.
	fn holds<T: 'static>(registry: &TypeRegistry, entity: &EntityBody) -> bool {
		entity.iter().any(|(name, _)| {
			DynamicTemplate::registration(registry, name).is_some_and(
				|registration| registration.type_id() == TypeId::of::<T>(),
			)
		})
	}

	/// The scene meta of `body`: its skeleton spawned in a throwaway world,
	/// every [`SceneMeta`] applied in body order, and the meta entity read
	/// back as a record of every component that is a [`LexiconDef`].
	fn scene_meta(
		types: &AppTypeRegistry,
		registry: &TypeRegistry,
		body: &SceneBody,
		path: &RelPath,
	) -> Result<EntityBodyRecord> {
		let mut world = World::new();
		world.insert_resource(types.clone());
		let keys = EntityKeys::new(body.entities.keys().cloned());
		let entities = body
			.entities
			.keys()
			.map(|key| (key.clone(), world.spawn_empty().id()))
			.collect::<HashMap<_, _>>();

		// the hierarchy first, so a meta component's hooks find its parents
		for (key, entity_body) in &body.entities {
			let parent = entity_body.iter().find_map(|(name, value)| {
				DynamicTemplate::registration(registry, name)
					.filter(|registration| {
						registration.type_info().is::<ChildOf>()
					})
					.and_then(|_| EntityKey::of_reference(value))
					.and_then(|parent| entities.get(&parent))
			});
			if let Some(parent) = parent {
				world.entity_mut(entities[key]).insert(ChildOf(*parent));
			}
		}

		// only the components taking part in compilation, so no other hook runs
		let mut applies = Vec::new();
		for (key, entity_body) in &body.entities {
			for (name, value) in entity_body.iter() {
				let Some(registration) =
					DynamicTemplate::registration(registry, name)
				else {
					continue;
				};
				let Some(apply) = registration.data::<ReflectSceneMeta>()
				else {
					continue;
				};
				if let Some(component) = registration.data::<ReflectComponent>()
				{
					let reflected = keys
						.deserialize(registration, registry, value)
						.map_err(|err| MetaError::new(key, err))?;
					component.insert(
						&mut world.entity_mut(entities[key]),
						reflected.as_ref(),
						registry,
					);
				}
				applies.push((key.clone(), entities[key], apply.clone()));
			}
		}

		let meta = world.spawn_empty().id();
		for (key, entity, apply) in applies {
			apply
				.apply(&mut SceneMetaContext {
					world: &mut world,
					body,
					document: path,
					entity,
					key: &key,
					meta,
				})
				.map_err(|err| MetaError::new(&key, err))?;
		}
		Self::record_of(&world, registry, meta, SceneRecord::COLLECTION)
	}

	/// The components of `entity` as a record of `collection`, each keyed by
	/// its [`LexiconDef`], in its serde form where it registers one and its
	/// reflected form otherwise, which is the same for plain data; one that is
	/// no def is left off.
	pub fn record_of(
		world: &World,
		registry: &TypeRegistry,
		entity: Entity,
		collection: Nsid,
	) -> Result<EntityBodyRecord> {
		let entity = world.entity(entity);
		let mut record = EntityBodyRecord::new(collection);
		for &component_id in entity.archetype().components().iter() {
			let Some(type_id) = world
				.components()
				.get_info(component_id)
				.and_then(|info| info.type_id())
			else {
				continue;
			};
			let Some(registration) = registry.get(type_id) else {
				continue;
			};
			let path = registration.type_info().type_path();
			let (Some(def), Some(component)) = (
				registration.data::<ReflectLexiconDef>(),
				registration.data::<ReflectComponent>(),
			) else {
				warn!(
					"`{path}` on a record's meta entity is no def, so it is \
					 left off: derive and reflect `LexiconDef`"
				);
				continue;
			};
			let Some(reflected) = component.reflect(entity) else {
				continue;
			};
			let value = Value::from_serde(&TypedReflectSerializer::new(
				reflected.as_partial_reflect(),
				registry,
			))?;
			record.insert_value(def.nsid().clone(), value.try_into()?);
		}
		record.xok()
	}
}

/// An apply's error and the key of the entity it was applied on, so the
/// error is placed where the author wrote it.
#[derive(Debug)]
struct MetaError {
	key: EntityKey,
	error: BevyError,
}

impl MetaError {
	fn new(key: &EntityKey, error: BevyError) -> BevyError {
		Self {
			key: key.clone(),
			error,
		}
		.into()
	}
}

impl core::fmt::Display for MetaError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "{}", self.error)
	}
}

impl core::error::Error for MetaError {}
