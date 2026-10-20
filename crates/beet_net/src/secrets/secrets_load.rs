//! Loading a `<Secrets>` declaration's document when its scene builds.

use crate::prelude::*;
use beet_core::prelude::*;

/// On `<Secrets>` insert, read its document through the store the
/// declaration resolves to and insert it, the plaintext index, on the entity
/// whatever this identity opens; then open every group the discovered
/// identity can, set its `EnvVar` records into the process environment
/// (existing wins) and insert the opened document as [`OpenSecrets`] beside
/// it.
///
/// The read parks a [`PendingGuard`] on the build root (or on this entity
/// outside a build), deferring [`Ready`] until the document has loaded, so a
/// load verb (`CallOnReady`) dispatching a deploy under the same root finds
/// the credentials in its environment. Any failure is one warning and
/// nothing inserted, never an error: a cloud box's repo store never carries
/// an identity, and a contributor without one must still build and run.
pub(crate) fn load_on_insert(
	ev: On<Insert<Secrets>>,
	declared: Query<&Secrets>,
	build_root: Option<Res<TemplateBuildRoot>>,
	mut commands: Commands,
) -> Result {
	let entity = ev.entity;
	let label = declared.get(entity)?.label.clone();
	let root = build_root.map(|root| **root);
	// one queued command parks the guard and spawns the load holding it, so
	// however the task ends the guard resolves (see `RoutesDir`)
	commands.queue(move |world: &mut World| {
		let guard = TemplatePending::park_on(
			world,
			root.unwrap_or(entity),
			PendingKind::Passive,
			format!("<Secrets label=\"{label}\"> load"),
		);
		let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
			return;
		};
		// local for the same reason the routes scan is: the load is bridge
		// heavy and a bridge poll only reliably completes on the local executor
		entity_mut.run_async_local(async move |entity: AsyncEntity| {
			if let Err(err) = load(&entity).await {
				warn!("secrets `{label}`: {err}");
			}
			entity
				.world()
				.with(move |world: &mut World| guard.resolve(world))
				.await;
		});
	});
	Ok(())
}

/// The load itself: the index, then the shared env var load, then
/// `OpenSecrets` and the resolved handle on the entity once no other document
/// in the world holds one of its names.
async fn load(entity: &AsyncEntity) -> Result {
	let handle = SecretsHandle::of_declaration(entity).await?;
	let Some(document) = handle.read_written().await? else {
		return OK;
	};
	// the index needs no identity: a render reads a record's metadata off it
	entity.insert(document.clone()).await?;
	let Some(opened) = handle.load_env_vars_of(&document)? else {
		return OK;
	};
	// one map across the world: a name in two documents is an error naming
	// both, never a silent shadow; a second declaration of the same file (an
	// entry checked from another entry's process) is the same document
	let mine = opened.clone();
	let mine_handle = handle.clone();
	let label = handle.label.clone().unwrap_or_default();
	entity
		.world()
		.with_state::<Query<(&Secrets, &SecretsHandle, &OpenSecrets)>, Result>(
			move |others| {
				for (secrets, other_handle, other) in
					others.iter().filter(|(_, other_handle, _)| {
						!mine_handle.same_file(other_handle)
					}) {
					let overlap = mine.overlap(other);
					if !overlap.is_empty() {
						bevybail!(
							"record(s) {} are declared in both `{label}` ({}) \
							and `{}` ({}): a name is loaded from exactly one \
							document",
							overlap
								.iter()
								.map(|name| format!("`{name}`"))
								.collect::<Vec<_>>()
								.join(", "),
							mine_handle.path,
							secrets.label,
							other_handle.path
						);
					}
				}
				Ok(())
			},
		)
		.await?;
	entity.insert((handle, opened)).await
}

impl SecretsHandle {
	/// The document, `None` when the file is not written yet (every entry's
	/// state before its first `set`, logged at debug).
	pub async fn read_written(&self) -> Result<Option<SecretsDocument>> {
		if !self.exists().await? {
			debug!(
				"document {} is not written yet: `secrets/set` writes a \
				document, `<SecretsExport>` an export",
				self.describe()
			);
			return None.xok();
		}
		self.read().await?.xmap(Some).xok()
	}

	/// Open `document` with the discovered identity: `None` when there is
	/// no identity to open it with (one warning naming the identity path and
	/// the file), else what this identity opened, verified against the
	/// index.
	pub fn open_discovered(
		&self,
		document: &SecretsDocument,
	) -> Result<Option<OpenSecrets>> {
		let Some(identities) = AgeIdentityFile::discover()? else {
			warn!(
				"no age identity at `{}` to open document {} with: its {} \
				record(s) are not loaded (`beet vault/keygen` makes one, \
				`vault/restore-identity` restores one)",
				AgeIdentityFile::default_path()?.display(),
				self.describe(),
				document.record_count()
			);
			return None.xok();
		};
		document.open(&identities)?.xmap(Some).xok()
	}

	/// [`read_written`](Self::read_written), then
	/// [`load_env_vars_of`](Self::load_env_vars_of): the load the launch runs
	/// before the entry builds.
	pub async fn load_env_vars(&self) -> Result<Option<OpenSecrets>> {
		match self.read_written().await? {
			Some(document) => self.load_env_vars_of(&document),
			None => None.xok(),
		}
	}

	/// [`open_discovered`](Self::open_discovered), then every `EnvVar`
	/// record into the process environment (existing wins), one line
	/// logged: what the launch and the `<Secrets>` load on ready both do with
	/// a document they read.
	pub fn load_env_vars_of(
		&self,
		document: &SecretsDocument,
	) -> Result<Option<OpenSecrets>> {
		let Some(opened) = self.open_discovered(document)? else {
			return None.xok();
		};
		let count = opened.set_env_vars()?;
		info!(
			"document {}: opened {} of {} group(s), {} record(s), {} env var(s) \
			set{}",
			self.describe(),
			opened.opened.len(),
			document.groups.len(),
			opened.secrets.len(),
			count,
			match opened.pending.is_empty() {
				true => String::new(),
				false => format!(
					"; group(s) {} list this identity but were sealed before it \
					was added: a member runs `secrets/rekey`",
					opened.pending.join(", ")
				),
			}
		);
		Some(opened).xok()
	}
}

#[cfg(test)]
mod test {
	use crate::vault::test_support::VerbWorld;
	use beet_core::prelude::*;

	/// A declared document loads on insert: the env var lands, the index and
	/// the opened document sit on the entity, and `Ready` waited for it.
	#[beet_core::test]
	async fn loads_a_declared_document() {
		let mut fixture = VerbWorld::new();
		let identities = fixture.identities();
		let mut document = SecretsDocument::default();
		document
			.set(
				&identities,
				"default",
				"BEET_TEST_SECRETS_LOAD",
				"loaded",
				SecretRecord {
					role: Some(SecretRole::EnvVar),
					..default()
				},
			)
			.unwrap();
		document
			.set(
				&identities,
				"default",
				"BEET_TEST_SECRETS_KEPT",
				"kept",
				default(),
			)
			.unwrap();
		fixture
			.secrets("secrets.toml")
			.write(&document)
			.await
			.unwrap();
		let entity = fixture
			.world
			.spawn((Secrets::default(), ChildOf(fixture.root)))
			.id();
		AsyncRunner::settle_async_tasks(&mut fixture.world).await;
		fixture
			.world
			.get::<SecretsDocument>(entity)
			.unwrap()
			.record_count()
			.xpect_eq(2);
		let opened = fixture.world.get::<OpenSecrets>(entity).unwrap();
		opened.secrets.len().xpect_eq(2);
		opened
			.get("BEET_TEST_SECRETS_KEPT")
			.unwrap()
			.value
			.as_str()
			.xpect_eq("kept");
		env_ext::var("BEET_TEST_SECRETS_LOAD")
			.unwrap()
			.xpect_eq("loaded");
		env_ext::var("BEET_TEST_SECRETS_KEPT").unwrap_err();
		// the guard resolved: nothing is pending on the entity
		fixture
			.world
			.get::<TemplatePending>(entity)
			.map(|pending| pending.is_empty())
			.unwrap_or(true)
			.xpect_true();
		// SAFETY: test-only, a name no other test reads
		unsafe {
			env_ext::remove_var("BEET_TEST_SECRETS_LOAD").unwrap();
		}
	}

	/// A name loaded from two files is refused naming both, while a second
	/// declaration of the same file is the same document and loads.
	#[beet_core::test]
	async fn one_name_loads_from_one_document() {
		let mut fixture = VerbWorld::new();
		let identities = fixture.identities();
		for file in ["a.toml", "b.toml"] {
			let mut document = SecretsDocument::default();
			document
				.set(
					&identities,
					"default",
					"BEET_TEST_SHARED",
					file,
					default(),
				)
				.unwrap();
			fixture.secrets(file).write(&document).await.unwrap();
		}
		let first = fixture
			.world
			.spawn((
				Secrets::new("a.toml").with_label("a"),
				ChildOf(fixture.root),
			))
			.id();
		AsyncRunner::settle_async_tasks(&mut fixture.world).await;
		fixture.world.get::<OpenSecrets>(first).xpect_some();
		// the same file under another label is the same document
		let again = fixture
			.world
			.spawn((
				Secrets::new("a.toml").with_label("a-again"),
				ChildOf(fixture.root),
			))
			.id();
		AsyncRunner::settle_async_tasks(&mut fixture.world).await;
		fixture.world.get::<OpenSecrets>(again).xpect_some();
		// another file holding the name is refused: nothing lands
		let other = fixture
			.world
			.spawn((
				Secrets::new("b.toml").with_label("b"),
				ChildOf(fixture.root),
			))
			.id();
		AsyncRunner::settle_async_tasks(&mut fixture.world).await;
		fixture.world.get::<OpenSecrets>(other).xpect_none();
	}

	/// A missing document is not an error: nothing is inserted and the load
	/// settles.
	#[beet_core::test]
	async fn a_missing_document_loads_nothing() {
		let mut fixture = VerbWorld::new();
		let entity = fixture
			.world
			.spawn((Secrets::new("missing.toml"), ChildOf(fixture.root)))
			.id();
		AsyncRunner::settle_async_tasks(&mut fixture.world).await;
		fixture.world.get::<SecretsDocument>(entity).xpect_none();
		fixture.world.get::<OpenSecrets>(entity).xpect_none();
	}
}
