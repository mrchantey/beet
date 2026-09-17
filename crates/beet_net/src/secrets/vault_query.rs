//! Resolving a vault by label or by path.

use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;

/// Resolves the vault a verb names (`--vault=<label or path>`) into a
/// [`VaultHandle`]: a declared `<Vault>` by label, with its store (the
/// `StoreRef` target's, else the nearest ancestor `BlobStore`) and its
/// recipients (its own, else the nearest ancestor [`AgeRecipients`]); or an
/// undeclared file by path or store uri, with no recipients declared. The
/// default label `.env` falls back to the undeclared `.env.age` beside the
/// entry when no `<Vault label=".env">` is declared.
#[derive(SystemParam)]
pub struct VaultQuery<'w, 's> {
	vaults: Query<'w, 's, (Entity, &'static Vault, Option<&'static StoreRef>)>,
	stores: Query<'w, 's, &'static BlobStore>,
	ancestor_stores: AncestorQuery<'w, 's, &'static BlobStore>,
	ancestor_recipients: AncestorQuery<'w, 's, &'static AgeRecipients>,
	repo: Query<'w, 's, &'static BlobStore, With<RepoStore>>,
}

/// What `--vault` names: a declared label, or a file by path or uri.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VaultSelector {
	/// A `<Vault label>`, the default label when none is given.
	Label(SmolStr),
	/// An undeclared file: a filesystem path (`~/personal.toml.age`,
	/// `dir/x.env.age`) or a store uri ending in the file
	/// (`s3://bucket/secrets/x.toml.age`).
	Path(String),
}

impl VaultSelector {
	/// A value is a path when it ends in `.age`, has a `/`, or names a
	/// scheme; anything else is a label. `None` is the default label.
	pub fn parse(value: Option<&str>) -> Self {
		let value = value.map(str::trim).unwrap_or(Vault::ENV_LABEL);
		match value.ends_with(".age") || value.contains(['/', ':']) {
			true => Self::Path(value.to_string()),
			false => Self::Label(SmolStr::new(value)),
		}
	}
}

impl VaultQuery<'_, '_> {
	/// The vault `selector` names from `caller`'s position.
	pub fn resolve(
		&self,
		caller: Entity,
		selector: Option<&str>,
	) -> Result<VaultHandle> {
		match VaultSelector::parse(selector) {
			VaultSelector::Label(label) => self.resolve_label(caller, &label),
			VaultSelector::Path(path) => Self::resolve_path(&path),
		}
	}

	/// The declared vault labelled `label`, or for the default label the
	/// undeclared `.env.age` beside the entry.
	pub fn resolve_label(
		&self,
		caller: Entity,
		label: &str,
	) -> Result<VaultHandle> {
		if let Some((entity, vault, store_ref)) = self
			.vaults
			.iter()
			.find(|(_, vault, _)| vault.label == label)
		{
			return self.handle(entity, vault, store_ref);
		}
		if label == Vault::ENV_LABEL {
			return VaultHandle::new(
				self.entry_store(caller)?,
				Vault::ENV_PATH,
			)?
			.with_label(Vault::ENV_LABEL)
			.xok();
		}
		let declared = self
			.vaults
			.iter()
			.map(|(_, vault, _)| format!("`{}`", vault.label))
			.collect::<Vec<_>>();
		bevybail!(
			"no `<Vault label=\"{label}\">` is declared{}; a file is named by \
			its path instead, ie `--vault=dir/{label}.toml.age`",
			match declared.is_empty() {
				true => String::new(),
				false => format!(" (declared: {})", declared.join(", ")),
			}
		)
	}

	/// An undeclared vault by filesystem path or store uri: the store is the
	/// file's directory (or the uri without its last segment), the file its
	/// last segment, and no recipients are declared.
	pub fn resolve_path(path: &str) -> Result<VaultHandle> {
		let (uri, file) = Self::split_uri(path)?;
		let store = StoreProvider::from_uri(&uri)?.into_blob_store();
		VaultHandle::new(store, file)
	}

	/// Every declared vault, resolved, with its label; a vault that fails to
	/// resolve reports why in its place, so `check` can list it.
	pub fn declared(&self) -> Vec<(SmolStr, Result<VaultHandle>)> {
		self.vaults
			.iter()
			.map(|(entity, vault, store_ref)| {
				(vault.label.clone(), self.handle(entity, vault, store_ref))
			})
			.collect()
	}

	/// The handle of a declared vault.
	fn handle(
		&self,
		entity: Entity,
		vault: &Vault,
		store_ref: Option<&StoreRef>,
	) -> Result<VaultHandle> {
		let store = match store_ref {
			Some(store_ref) => {
				self.stores.get(store_ref.store()).cloned().map_err(|_| {
					bevyhow!(
						"vault `{}` names store entity {} which carries no \
						`BlobStore`: give the declaration a store provider, ie \
						`<FsStore/>`",
						vault.label,
						store_ref.store()
					)
				})?
			}
			None => self.entry_store(entity)?,
		};
		let recipients = match vault.recipients.is_empty() {
			false => vault.recipients.clone(),
			true => self
				.ancestor_recipients
				.get(entity)
				.map(|recipients| recipients.0.clone())
				.unwrap_or_default(),
		};
		VaultHandle::new(store, vault.path.as_str())?
			.with_label(vault.label.clone())
			.with_recipients(recipients)
			.xok()
	}

	/// The store a relative vault path resolves in: the nearest ancestor
	/// `BlobStore` of `entity`, else the repo store wherever it sits.
	fn entry_store(&self, entity: Entity) -> Result<BlobStore> {
		self.ancestor_stores
			.get(entity)
			.ok()
			.or_else(|| self.repo.single().ok())
			.cloned()
			.ok_or_else(|| {
				bevyhow!(
					"no store to resolve a vault path in: entity {entity} has no \
					ancestor `BlobStore` and the world has no repo store"
				)
			})
	}

	/// Split a path or uri into the store uri of its directory and its file
	/// name: `~/a/b.toml.age` is `fs:~/a` + `b.toml.age`,
	/// `s3://bucket/dir/b.toml.age?region=x` is `s3://bucket/dir?region=x` +
	/// `b.toml.age`, and a bare `b.toml.age` is the cwd.
	fn split_uri(path: &str) -> Result<(StoreUri, String)> {
		let path = path.trim();
		let (base, query) = path
			.split_once('?')
			.map(|(base, query)| (base, format!("?{query}")))
			.unwrap_or((path, String::new()));
		let is_uri = base.contains("://") || base.starts_with("fs:");
		let (dir, file) = match base.rsplit_once('/') {
			// `fs:` alone is the cwd
			Some((dir, file))
				if !dir.ends_with(':') && !dir.ends_with(":/") =>
			{
				(dir.to_string(), file)
			}
			Some(_) => bevybail!(
				"`{path}` names no file: a vault uri is a store followed by the \
				file within it, ie `s3://bucket/secrets/x.toml.age`"
			),
			None => match is_uri {
				true => ("fs".to_string(), base.trim_start_matches("fs:")),
				false => (".".to_string(), base),
			},
		};
		if file.is_empty() {
			bevybail!("`{path}` names no file");
		}
		let uri = match is_uri {
			true => StoreUri::parse(&format!("{dir}{query}"))?,
			false => StoreUri::parse(&format!("fs:{dir}"))?,
		};
		(uri, file.to_string()).xok()
	}
}

impl VaultHandle {
	/// The vault `selector` names from `caller`'s position, the resolution
	/// every vault verb starts from.
	pub async fn resolve(
		caller: &AsyncEntity,
		selector: Option<&str>,
	) -> Result<Self> {
		let selector = selector.map(SmolStr::new);
		caller
			.with_state::<VaultQuery, _>(move |entity, query| {
				query.resolve(entity, selector.as_deref())
			})
			.await?
	}

	/// Every declared vault, see [`VaultQuery::declared`].
	pub async fn declared(
		caller: &AsyncEntity,
	) -> Result<Vec<(SmolStr, Result<Self>)>> {
		caller
			.with_state::<VaultQuery, _>(|_, query| query.declared())
			.await
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A recipient with no identity behind it.
	fn recipient() -> AgeRecipient { AgeIdentity::generate().to_recipient() }

	/// An entry root carrying the repo store, declaring vaults under a stack
	/// with inherited recipients: the shape a deploy entry has.
	fn world_with_vaults() -> (World, Entity, AgeRecipient, AgeRecipient) {
		let mut world = StorePlugin.into_world();
		let inherited = recipient();
		let own = recipient();
		// a declaration with no store provider on it
		let storeless = world.spawn_empty().id();
		let root = world
			.spawn((BlobStore::temp(), RepoStore, children![
				(AgeRecipients(vec![inherited.clone()]), children![
					Vault::new("infra/secrets/mail.toml.age")
						.with_label("mail-prod"),
					Vault::new("personal.json.age")
						.with_label("personal")
						.with_recipients([own.clone()]),
					(
						Vault::new("secrets").with_label("cold"),
						StoreRef(storeless)
					),
				]),
				Vault::new("orphan.env.age").with_label("orphan"),
			]))
			.id();
		world.flush();
		(world, root, inherited, own)
	}

	#[beet_core::test]
	fn resolves_a_label_with_inherited_recipients() {
		let (mut world, root, inherited, own) = world_with_vaults();
		world.with_state::<VaultQuery, _>(|query| {
			let mail = query.resolve(root, Some("mail-prod")).unwrap();
			mail.path.as_str().xpect_eq("infra/secrets/mail.toml.age");
			mail.format.xpect_eq(VaultFormat::Toml);
			mail.recipients.unwrap().xpect_eq(vec![inherited]);
			// its own list wins over the ancestor's
			query
				.resolve(root, Some("personal"))
				.unwrap()
				.recipients
				.unwrap()
				.xpect_eq(vec![own]);
			// no list anywhere: declared but empty, which `write` refuses
			query
				.resolve(root, Some("orphan"))
				.unwrap()
				.recipients
				.unwrap()
				.xpect_eq(vec![]);
		});
	}

	#[beet_core::test]
	fn the_env_default_is_the_file_beside_the_entry() {
		let (mut world, root, ..) = world_with_vaults();
		world.with_state::<VaultQuery, _>(|query| {
			let env = query.resolve(root, None).unwrap();
			env.label.unwrap().as_str().xpect_eq(".env");
			env.path.as_str().xpect_eq(".env.age");
			env.recipients.xpect_none();
		});
	}

	#[beet_core::test]
	fn errors_name_the_missing_declaration() {
		let (mut world, root, ..) = world_with_vaults();
		world.with_state::<VaultQuery, _>(|query| {
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
		});
	}

	#[beet_core::test]
	fn resolves_a_path_or_uri() {
		let vault =
			VaultQuery::resolve_path("memory://vaults/dir/x.toml.age").unwrap();
		vault.path.as_str().xpect_eq("x.toml.age");
		vault.store.subdir().as_str().xpect_eq("dir");
		vault.recipients.xpect_none();
		VaultQuery::resolve_path("/tmp/beet/personal.env.age")
			.unwrap()
			.store
			.base_dir()
			.unwrap()
			.to_string()
			.xpect_eq("/tmp/beet");
		VaultQuery::resolve_path("x.json.age")
			.unwrap()
			.store
			.id()
			.xpect_eq("fs");
		VaultQuery::resolve_path("memory://x.age").unwrap_err();
		VaultQuery::resolve_path("dir/x.txt").unwrap_err();
	}

	#[beet_core::test]
	fn selector_tells_labels_from_paths() {
		VaultSelector::parse(None)
			.xpect_eq(VaultSelector::Label(".env".into()));
		VaultSelector::parse(Some("mail-prod"))
			.xpect_eq(VaultSelector::Label("mail-prod".into()));
		VaultSelector::parse(Some(".env.age"))
			.xpect_eq(VaultSelector::Path(".env.age".into()));
		VaultSelector::parse(Some("~/p.toml.age"))
			.xpect_eq(VaultSelector::Path("~/p.toml.age".into()));
	}
}
