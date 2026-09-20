//! Loading a `<Secrets>` declaration's document when its scene builds.

use crate::prelude::*;
use beet_core::prelude::*;

/// On `<Secrets>` insert, read its document through the store the
/// declaration resolves to, open every group the discovered identity can,
/// set its `EnvVar` records into the process environment (existing wins)
/// and insert the opened document as [`OpenSecrets`] on the entity.
///
/// The read parks a [`PendingGuard`] on the build root (or on this entity
/// outside a build), deferring [`Ready`] until the document has loaded, so a
/// load verb (`CallOnReady`) dispatching a deploy under the same root finds
/// the credentials in its environment. Any failure is one warning and
/// nothing inserted, never an error: a cloud box's repo store never carries
/// an identity, and a contributor without one must still build and run.
pub(crate) fn load_on_insert(
	ev: On<Insert, Secrets>,
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

/// The load itself: the shared env var load, then `OpenSecrets` on the
/// entity once no other document in the world holds one of its names.
async fn load(entity: &AsyncEntity) -> Result {
	let handle = SecretsHandle::of_declaration(entity).await?;
	let Some(opened) = handle.load_env_vars().await? else {
		return OK;
	};
	// one map across the world: a name in two documents is an error naming
	// both, never a silent shadow
	let mine = opened.clone();
	let label = handle.label.clone().unwrap_or_default();
	entity
		.world()
		.with_state::<Query<(&Secrets, &OpenSecrets)>, Result>(move |others| {
			for (secrets, other) in others.iter() {
				let overlap = mine.overlap(other);
				if !overlap.is_empty() {
					bevybail!(
						"record(s) {} are declared in both `{label}` and `{}`: a \
						name is loaded from exactly one document",
						overlap
							.iter()
							.map(|name| format!("`{name}`"))
							.collect::<Vec<_>>()
							.join(", "),
						secrets.label
					);
				}
			}
			Ok(())
		})
		.await?;
	entity.insert(opened).await
}

impl SecretsHandle {
	/// Read the document and open it with the discovered identity: `None`
	/// when the file is not written yet (every entry's state before its
	/// first `set`, logged at debug) or there is no identity to open it
	/// with (one warning naming the identity path and the file), else the
	/// document and what this identity opened, verified against the index.
	pub async fn open_discovered(
		&self,
	) -> Result<Option<(SecretsDocument, OpenSecrets)>> {
		if !self.exists().await? {
			debug!(
				"document {} is not written yet: `secrets/set` writes a \
				document, `<SecretsExport>` an export",
				self.describe()
			);
			return None.xok();
		}
		let document = self.read().await?;
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
		let opened = document.open(&identities)?;
		Some((document, opened)).xok()
	}

	/// [`open_discovered`](Self::open_discovered), then every `EnvVar`
	/// record into the process environment (existing wins), one line
	/// logged: the load the launch runs before the entry builds and the
	/// `<Secrets>` load repeats on ready.
	pub async fn load_env_vars(&self) -> Result<Option<OpenSecrets>> {
		let Some((document, opened)) = self.open_discovered().await? else {
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

	/// A declared document loads on insert: the env var lands, the opened
	/// document sits on the entity, and `Ready` waited for it.
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
		fixture.world.get::<OpenSecrets>(entity).xpect_none();
	}
}
