//! Resolving a secrets document by label or by path.

use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;

/// Resolves the document a verb names (`--vault=<label or path>`) into a
/// [`VaultHandle`]: a declared `<Secrets>` by label, with its store (the
/// `StoreRef` target's, else the nearest ancestor `BlobStore`, else the repo
/// store); or an undeclared file by path or store uri. Omitted, it is the
/// one declared document, an error naming the labels when several are
/// declared, or the undeclared `secrets.toml.age` beside the entry when none
/// is.
#[derive(SystemParam)]
pub struct SecretsQuery<'w, 's> {
	declared:
		Query<'w, 's, (Entity, &'static Secrets, Option<&'static StoreRef>)>,
	stores: Query<'w, 's, &'static BlobStore>,
	ancestor_stores: AncestorQuery<'w, 's, &'static BlobStore>,
	repo: Query<'w, 's, &'static BlobStore, With<RepoStore>>,
}

/// What `--vault` names: the default, a declared label, or a file by path
/// or uri.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentSelector {
	/// No `--vault`: the one declared document, or the conventional file.
	Default,
	/// A `<Secrets label>`.
	Label(SmolStr),
	/// An undeclared file: a filesystem path (`~/personal.toml.age`,
	/// `dir/x.toml.age`) or a store uri ending in the file
	/// (`s3://bucket/secrets/x.toml.age`).
	Path(String),
}

impl DocumentSelector {
	/// A value is a path when it ends in `.age`, has a `/`, or names a
	/// scheme; anything else is a label. `None` is the default.
	pub fn parse(value: Option<&str>) -> Self {
		let Some(value) =
			value.map(str::trim).filter(|value| !value.is_empty())
		else {
			return Self::Default;
		};
		match value.ends_with(VaultHandle::SUFFIX) || value.contains(['/', ':'])
		{
			true => Self::Path(value.to_string()),
			false => Self::Label(SmolStr::new(value)),
		}
	}
}

impl SecretsQuery<'_, '_> {
	/// The document `selector` names from `caller`'s position.
	pub fn resolve(
		&self,
		caller: Entity,
		selector: Option<&str>,
	) -> Result<VaultHandle> {
		match DocumentSelector::parse(selector) {
			DocumentSelector::Default => self.resolve_default(caller),
			DocumentSelector::Label(label) => self.resolve_label(&label),
			DocumentSelector::Path(path) => VaultHandle::from_uri(&path),
		}
	}

	/// The default document: the one declared, an error naming the labels
	/// when several are, or the undeclared conventional file in `caller`'s
	/// entry store when none is.
	pub fn resolve_default(&self, caller: Entity) -> Result<VaultHandle> {
		let mut declared = self.declared.iter();
		match (declared.next(), declared.next()) {
			(Some((entity, secrets, store_ref)), None) => {
				self.handle(entity, secrets, store_ref)
			}
			(Some(_), Some(_)) => bevybail!(
				"several documents are declared ({}): name one with \
				`--vault=<label>`",
				self.labels()
			),
			(None, _) => VaultHandle::new(
				self.entry_store(caller)?,
				SecretsDocument::DEFAULT_PATH,
			)?
			.with_label(Secrets::DEFAULT_LABEL)
			.xok(),
		}
	}

	/// The declared document labelled `label`, an error naming the declared
	/// labels otherwise.
	pub fn resolve_label(&self, label: &str) -> Result<VaultHandle> {
		self.declared
			.iter()
			.find(|(_, secrets, _)| secrets.label == label)
			.map(|(entity, secrets, store_ref)| {
				self.handle(entity, secrets, store_ref)
			})
			.unwrap_or_else(|| {
				bevybail!(
					"no `<Secrets label=\"{label}\">` is declared{}; a file is \
					named by its path instead, ie `--vault=dir/{label}.toml.age`",
					match self.declared.is_empty() {
						true => String::new(),
						false => format!(" (declared: {})", self.labels()),
					}
				)
			})
	}

	/// Every declared document, resolved, with its label; one that fails to
	/// resolve reports why in its place, so `check` can list it.
	pub fn declared(&self) -> Vec<(SmolStr, Result<VaultHandle>)> {
		self.declared
			.iter()
			.map(|(entity, secrets, store_ref)| {
				(
					secrets.label.clone(),
					self.handle(entity, secrets, store_ref),
				)
			})
			.collect()
	}

	/// The handle of the declaration on `entity`.
	pub fn handle_of(&self, entity: Entity) -> Result<VaultHandle> {
		let (entity, secrets, store_ref) = self.declared.get(entity)?;
		self.handle(entity, secrets, store_ref)
	}

	/// The declared labels, for a message.
	fn labels(&self) -> String {
		self.declared
			.iter()
			.map(|(_, secrets, _)| format!("`{}`", secrets.label))
			.collect::<Vec<_>>()
			.join(", ")
	}

	/// The handle of a declared document.
	fn handle(
		&self,
		entity: Entity,
		secrets: &Secrets,
		store_ref: Option<&StoreRef>,
	) -> Result<VaultHandle> {
		let store = match store_ref {
			Some(store_ref) => {
				self.stores.get(store_ref.store()).cloned().map_err(|_| {
					bevyhow!(
						"document `{}` names store entity {} which carries no \
						`BlobStore`: give the declaration a store provider, ie \
						`<FsStore/>`",
						secrets.label,
						store_ref.store()
					)
				})?
			}
			None => self.entry_store(entity)?,
		};
		VaultHandle::new(store, secrets.path.as_str())?
			.with_label(secrets.label.clone())
			.xok()
	}

	/// The store a relative document path resolves in: the nearest ancestor
	/// `BlobStore` of `entity`, else the repo store wherever it sits.
	fn entry_store(&self, entity: Entity) -> Result<BlobStore> {
		self.ancestor_stores
			.get(entity)
			.ok()
			.or_else(|| self.repo.single().ok())
			.cloned()
			.ok_or_else(|| {
				bevyhow!(
					"no store to resolve a document path in: entity {entity} \
					has no ancestor `BlobStore` and the world has no repo store"
				)
			})
	}
}

impl VaultHandle {
	/// The document `selector` names from `caller`'s position, the
	/// resolution every document verb starts from.
	pub async fn resolve_document(
		caller: &AsyncEntity,
		selector: Option<&str>,
	) -> Result<Self> {
		let selector = selector.map(SmolStr::new);
		caller
			.with_state::<SecretsQuery, _>(move |entity, query| {
				query.resolve(entity, selector.as_deref())
			})
			.await?
	}

	/// Every declared document, see [`SecretsQuery::declared`].
	pub async fn declared(
		caller: &AsyncEntity,
	) -> Result<Vec<(SmolStr, Result<Self>)>> {
		caller
			.with_state::<SecretsQuery, _>(|_, query| query.declared())
			.await
	}

	/// The document the `<Secrets>` on `entity` declares, its `StoreRef`
	/// target's store awaited through the command queue where it lands.
	pub async fn of_declaration(entity: &AsyncEntity) -> Result<Self> {
		let target = entity
			.get::<StoreRef, _>(|store_ref| store_ref.store())
			.await
			.ok();
		let Some(target) = target else {
			return entity
				.with_state::<SecretsQuery, _>(|entity, query| {
					query.handle_of(entity)
				})
				.await?;
		};
		let secrets = entity.get::<Secrets, _>(Clone::clone).await?;
		let store =
			StoreRef::resolve::<BlobStore>(entity.world(), target).await?;
		Self::new(store, secrets.path.as_str())?
			.with_label(secrets.label)
			.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// An entry root carrying the repo store, declaring documents: the shape
	/// a deploy entry has.
	fn world_with_documents() -> (World, Entity) {
		let mut world = StorePlugin.into_world();
		// a declaration with no store provider on it
		let storeless = world.spawn_empty().id();
		let root = world
			.spawn((BlobStore::temp(), RepoStore, children![
				Secrets::new("infra/secrets/mail.toml.age")
					.with_label("mail-prod"),
				(
					Secrets::new("secrets").with_label("cold"),
					StoreRef(storeless)
				),
			]))
			.id();
		world.flush();
		(world, root)
	}

	#[beet_core::test]
	fn resolves_labels() {
		let (mut world, root) = world_with_documents();
		world.with_state::<SecretsQuery, _>(|query| {
			let mail = query.resolve(root, Some("mail-prod")).unwrap();
			mail.path.as_str().xpect_eq("infra/secrets/mail.toml.age");
			mail.label.clone().unwrap().as_str().xpect_eq("mail-prod");
			mail.describe()
				.xpect_eq("`mail-prod` (infra/secrets/mail.toml.age)");
			// several declared: the default needs a label
			query
				.resolve(root, None)
				.unwrap_err()
				.to_string()
				.xpect_contains("`mail-prod`")
				.xpect_contains("`cold`");
			query
				.resolve(root, Some("nope"))
				.unwrap_err()
				.to_string()
				.xpect_contains("`mail-prod`");
			// a `StoreRef` to an entity carrying no store
			query
				.resolve(root, Some("cold"))
				.unwrap_err()
				.to_string()
				.xpect_contains("BlobStore");
			query.declared().len().xpect_eq(2);
		});
	}

	#[beet_core::test]
	fn the_default_is_the_lone_declaration_or_the_conventional_file() {
		let mut world = StorePlugin.into_world();
		let root = world.spawn((BlobStore::temp(), RepoStore)).id();
		world.flush();
		world.with_state::<SecretsQuery, _>(|query| {
			let handle = query.resolve(root, None).unwrap();
			handle.path.as_str().xpect_eq("secrets.toml.age");
			handle.label.unwrap().as_str().xpect_eq("secrets");
		});
		world.spawn((
			Secrets::new("x.json.age").with_label("only"),
			ChildOf(root),
		));
		world.flush();
		world.with_state::<SecretsQuery, _>(|query| {
			query
				.resolve(root, None)
				.unwrap()
				.path
				.as_str()
				.xpect_eq("x.json.age");
		});
	}

	#[beet_core::test]
	fn selector_tells_labels_from_paths() {
		DocumentSelector::parse(None).xpect_eq(DocumentSelector::Default);
		DocumentSelector::parse(Some("")).xpect_eq(DocumentSelector::Default);
		DocumentSelector::parse(Some("mail-prod"))
			.xpect_eq(DocumentSelector::Label("mail-prod".into()));
		DocumentSelector::parse(Some("secrets.toml.age"))
			.xpect_eq(DocumentSelector::Path("secrets.toml.age".into()));
		DocumentSelector::parse(Some("~/p.toml.age"))
			.xpect_eq(DocumentSelector::Path("~/p.toml.age".into()));
	}
}
