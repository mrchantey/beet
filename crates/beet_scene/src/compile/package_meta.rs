//! Package meta: what needs the whole branch compiled, every scene's meta and
//! the dependencies, which no scene's own meta may see.
use super::*;
use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use beet_core::prelude::*;
use bevy_reflect::TypeRegistry;

/// Every scene in reach of one package compile, by its document's path: this
/// package's own, and each dependency's under its prefix.
pub(super) struct SceneSet {
	/// The record of every scene, as its document's path names it.
	pub records: BTreeMap<RelPath, EntityBodyRecord>,
	/// The files, this package's and each dependency's under its prefix.
	pub files: Files,
}

impl SceneSet {
	/// The scene meta `T` of the document at `path`.
	fn meta<T: LexiconDef + DeserializeOwned>(
		&self,
		path: &RelPath,
	) -> Result<Option<T>> {
		match self.records.get(path) {
			Some(record) => record.try_get::<T>(),
			None => None.xok(),
		}
	}

	/// The paths `path` imports, an empty set for a plain file.
	fn imports(&self, path: &RelPath) -> Result<Imports> {
		self.meta::<Imports>(path).map(Option::unwrap_or_default)
	}

	/// Check every use's props against its target's [`Props`]: each passed
	/// prop declared and of its kind, and each prop with no default passed. A
	/// target declaring no props has no interface to check, as a template
	/// with no schema is left unverified; one prop declared makes it strict.
	pub fn check_props(&self, documents: &[CompiledDocument]) -> Result {
		for document in documents {
			for use_site in &document.uses {
				let Some(props) = self.meta::<Props>(&use_site.document)?
				else {
					continue;
				};
				let located = |message: String| {
					bevyhow!(
						"{}: `{}` {message}",
						document.position(&use_site.key),
						use_site.document
					)
				};
				for (name, value) in use_site.props.iter() {
					let Some(schema) = props.0.get(name.as_str()) else {
						return Err(located(format!(
							"takes no prop `{name}`: it declares {}",
							Self::listed(props.0.keys())
						)));
					};
					// a binding resolves at runtime, its kind its source's
					if !Self::is_binding(value) && !schema.admits(value) {
						return Err(located(format!(
							"takes `{name}` as `{}`, not {}",
							schema.kind,
							value.kind()
						)));
					}
				}
				if let Some((name, schema)) =
					props.0.iter().find(|(name, schema)| {
						schema.default.is_none()
							&& !use_site.props.contains(name.as_str())
					}) {
					return Err(located(format!(
						"requires the prop `{name}`, a `{}`",
						schema.kind
					)));
				}
			}
		}
		Ok(())
	}

	/// Whether `value` is a binding a body holds in a prop's place, ie
	/// `@doc:name`, resolved at runtime.
	fn is_binding(value: &Value) -> bool {
		match value {
			Value::Str(text) => text.starts_with('@'),
			Value::Map(map) => map.keys().any(|key| key.starts_with('@')),
			_ => false,
		}
	}

	fn listed<'a>(names: impl Iterator<Item = &'a SmolStr>) -> String {
		let names = names.map(|name| format!("`{name}`")).collect::<Vec<_>>();
		match names.is_empty() {
			true => "none".into(),
			false => names.join(", "),
		}
	}

	/// Refuse a cycle through [`Instance`] edges, lazy or not: activation is
	/// eager once a set has loaded, so a cycle would never finish building.
	pub fn check_cycles(&self) -> Result {
		let mut done = BTreeSet::new();
		for path in self.records.keys() {
			self.visit(path, &mut Vec::new(), &mut done)?;
		}
		Ok(())
	}

	fn visit(
		&self,
		path: &RelPath,
		stack: &mut Vec<RelPath>,
		done: &mut BTreeSet<RelPath>,
	) -> Result {
		if done.contains(path) {
			return Ok(());
		}
		if let Some(start) = stack.iter().position(|item| item == path) {
			let cycle = stack[start..]
				.iter()
				.chain([path])
				.map(|path| format!("`{path}`"))
				.collect::<Vec<_>>()
				.join(" instantiates ");
			bevybail!("an instance cycle: {cycle}");
		}
		stack.push(path.clone());
		for (target, edge) in self.imports(path)?.0 {
			if edge.instance {
				self.visit(&target, stack, done)?;
			}
		}
		stack.pop();
		done.insert(path.clone());
		Ok(())
	}

	/// One [`LoadSet`] per document of this package: the document and its
	/// closure through eager and preload edges, stopping at a lazy one, with
	/// the union of every member's [`RequireCfg`].
	pub fn load_sets(&self, documents: &[RelPath]) -> Result<LoadSets> {
		let mut sets = LoadSets::default();
		for root in documents {
			let mut set = LoadSet::default();
			let mut requires = Vec::new();
			let mut seen = BTreeSet::new();
			let mut queue = vec![root.clone()];
			while let Some(path) = queue.pop() {
				if !seen.insert(path.clone()) {
					continue;
				}
				match self.records.get(&path) {
					Some(record) => {
						set.scenes.push(record.get::<BlobRef>()?.cid);
						if let Some(require) = record.try_get::<RequireCfg>()? {
							requires.push(require.cfg);
						}
						// visited in reverse so the stack pops in path order
						for (target, edge) in
							self.imports(&path)?.0.into_iter().rev()
						{
							if edge.kind != ImportKind::Lazy {
								queue.push(target);
							}
						}
					}
					None => {
						if let Some(blob) = self.files.0.get(&path) {
							set.blobs.push(blob.cid.clone());
						}
					}
				}
			}
			set.requires =
				RequireCfg::union(requires.iter().map(SmolStr::as_str))?
					.map(|require| require.cfg);
			sets.0.insert(root.clone(), set);
		}
		sets.xok()
	}

	/// Run every [`PackageMeta`] a scene of this package carries onto the
	/// package's meta entity in `world`, in path then key order.
	pub fn apply_package_meta(
		&self,
		world: &mut World,
		registry: &TypeRegistry,
		documents: &[RelPath],
		meta: Entity,
	) -> Result {
		for document in documents {
			let Some(record) = self.records.get(document) else {
				continue;
			};
			let applies = record
				.components()
				.filter_map(|(nsid, _)| {
					ReflectLexiconDef::registration(registry, nsid)
				})
				.filter_map(|registration| {
					registration.data::<ReflectPackageMeta>().cloned()
				})
				.collect::<Vec<_>>();
			for apply in applies {
				apply
					.apply(&mut PackageMetaContext {
						world,
						files: &self.files,
						scenes: &self.records,
						document,
						meta,
					})
					.map_err(|err| bevyhow!("{document}: {err}"))?;
			}
		}
		Ok(())
	}
}
