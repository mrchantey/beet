//! A document of another format grafted into the one it is written in.
use super::lowering::*;
use crate::body::EntityKeys;
use crate::body::KeySentinels;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::entity::EntityMapper;
use bevy::reflect::TypePath;

impl Lowering<'_> {
	/// `embedded` read in its format against the registry this lowers with,
	/// configured by its params, and its error and spans placed in the
	/// document holding it.
	pub fn read_embedded(
		&self,
		embedded: &Spanned<EmbeddedDocument>,
	) -> Result<ParsedDocument> {
		let params = FenceParams::new(embedded.params.clone());
		let bytes =
			MediaBytes::new_str(embedded.media_type.clone(), &embedded.source);
		let mut document =
			self.parsers.parse_fence(&bytes, &params).map_err(|error| {
				match (error.into_source(), embedded.start) {
					// where in the document, when the source stands there whole
					(error, Some(start)) => SourceError::within(error, start),
					(error, None) => {
						embedded.span.locate(SourceError::split(error).1)
					}
				}
			})?;
		match embedded.start {
			Some(start) => document.spans.shift(start as u32),
			None => document.spans = default(),
		}
		document.xok()
	}

	/// The key of an embedded document under `parent`: as an element's, by the
	/// tag its root is.
	pub fn embedded_key(
		document: &ParsedDocument,
		parent: &EntityKey,
		counts: &mut HashMap<SmolStr, usize>,
	) -> Result<EntityKey> {
		let tag = document
			.body
			.entities
			.first()
			.and_then(|(_, root)| root.get_typed::<Element>().ok())
			.map(|element| SmolStr::new(element.tag()))
			.unwrap_or_else(|| "#body".into());
		Self::counted_key(&tag, parent, counts)
	}

	/// Graft `body`, its root the entity keyed `key` and every other entity
	/// keyed under it, each reference within it following its target.
	pub fn embedded(
		&mut self,
		document: &ParsedDocument,
		key: &EntityKey,
	) -> Result {
		let body = &document.body;
		let graft = Graft::new(body, key)?;
		for (name, value) in body.resources.iter() {
			self.body.resources.insert(name.clone(), value.clone());
		}
		for (index, (source, components)) in body.entities.iter().enumerate() {
			let target = graft.key(source)?;
			if index > 0 {
				self.push(target.clone(), default(), &default())?;
			}
			if let Some(span) = document.spans.get(source) {
				self.spans.insert(target.clone(), span.clone());
			}
			for (name, value) in components.iter() {
				let value = self.grafted(&graft, name, value)?;
				self.entity(&target).insert(name.clone(), value);
			}
		}
		Ok(())
	}

	/// The component `value` keyed `name` with its references grafted: those
	/// of the hierarchy by name, as any other by the type's own
	/// `map_entities`.
	fn grafted(
		&self,
		graft: &Graft,
		name: &str,
		value: &Value,
	) -> Result<Value> {
		if [ChildOf::type_path(), AttributeOf::type_path()].contains(&name) {
			return match EntityKey::of_reference(value) {
				Some(source) => graft.key(&source)?.reference().xok(),
				None => value.clone().xok(),
			};
		}
		let Some((registration, map_entities)) = DynamicTemplate::registration(
			self.types, name,
		)
		.and_then(|registration| {
			registration
				.data::<ReflectMapEntities>()
				.map(|map_entities| (registration, map_entities))
		}) else {
			return value.clone().xok();
		};
		let keys = EntityKeys::new(graft.body.entities.keys().cloned());
		let mut reflected =
			keys.deserialize(registration, self.types, value)?;
		let mut mapper = GraftMapper {
			graft,
			keys: &keys,
			sentinels: default(),
			error: None,
		};
		map_entities.map_entities(reflected.as_mut(), &mut mapper);
		if let Some(error) = mapper.error {
			return Err(error);
		}
		mapper.sentinels.keyed(reflect_value_ext::patch_to_value(
			reflected.as_ref(),
			self.types,
		)?)
	}
}

/// Where each entity of an embedded body is keyed.
struct Graft<'a> {
	body: &'a SceneBody,
	/// The body's own root.
	source: EntityKey,
	/// What its root is keyed here.
	target: &'a EntityKey,
}

impl<'a> Graft<'a> {
	fn new(body: &'a SceneBody, target: &'a EntityKey) -> Result<Self> {
		let Some((source, _)) = body.entities.first() else {
			bevybail!("an embedded document has no root");
		};
		Self {
			body,
			source: source.clone(),
			target,
		}
		.xok()
	}

	/// The key of the entity the body keys `source`: the root's, under it for
	/// one keyed under the body's root, as for one keyed by a name.
	fn key(&self, source: &EntityKey) -> Result<EntityKey> {
		if *source == self.source {
			return self.target.clone().xok();
		}
		let under = source
			.as_str()
			.strip_prefix(self.source.as_str())
			.and_then(|rest| rest.strip_prefix('/'))
			.unwrap_or(source.as_str());
		EntityKey::parse(&format!("{}/{under}", self.target))
	}
}

/// Maps an embedded body's in-template entities to sentinels of their grafted
/// keys.
struct GraftMapper<'a> {
	graft: &'a Graft<'a>,
	keys: &'a EntityKeys,
	sentinels: KeySentinels,
	error: Option<BevyError>,
}

impl EntityMapper for GraftMapper<'_> {
	fn get_mapped(&mut self, source: Entity) -> Entity {
		let Some(key) = self.keys.key(source) else {
			return source;
		};
		match self.graft.key(&key) {
			Ok(key) => self.sentinels.entity(key),
			Err(error) => {
				self.error.get_or_insert(error);
				source
			}
		}
	}

	fn set_mapped(&mut self, _source: Entity, _target: Entity) {}
}
