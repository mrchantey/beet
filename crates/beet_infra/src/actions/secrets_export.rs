//! A stack's secret store written into a secrets document.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<SecretsExport document={$repo_export}/>` — write every secret the
/// stack's store holds into the secrets document `document` declares, keyed
/// by label, so a destroyed stack's secrets survive in git (the committed
/// repo document) and, with `dated=true`, as a dated series in the other
/// vendor's bucket.
///
/// The document is one of the same type a human keeps by hand: an index of
/// records (each label with its note, its provider address and when it was
/// last written) and one age blob sealed to `group`'s recipients, copied from
/// the entry's own document so the humans are listed once. Its `origin` says
/// which stack, provider and region it came from. Most of what a stack holds
/// is re-mintable, but the sovereign DKIM private key is not (its public
/// half is under a selector every receiver has cached) and a relay
/// credential a human enrolled by hand is re-mintable only by that human, so
/// the export takes the whole store rather than a list: a secret added later
/// is exported without anyone remembering to add it here.
///
/// Not dated, the export overwrites the declared path, and skips the write
/// when nothing but the timestamp changed so a no-op deploy is a no-op
/// commit. Dated, it writes `<dir>/YYYY/MM/DD/HHMMSSZ.<ext>` beside the
/// declared path, the shape a snapshot series has so one listing reads
/// either. It runs at the end of every deploy and provision, which is when a
/// secret changes, so the export is complete at the moment it matters. The
/// plaintext never touches the disk: the blob is sealed in memory and only
/// the document is written. `<SecretsRestore/>` is the inverse.
///
/// ```bsx
/// <Secrets bx:ref="repo_export" label="mail-prod" path="infra/secrets/mail--prod.toml"/>
/// <Secrets bx:ref="cold_export" label="mail-cold" path="secrets/export.toml" {StoreRef($cold_backups)}/>
/// <SecretsExport document={$repo_export}/>
/// <SecretsExport document={$cold_export} dated=true/>
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
pub async fn SecretsExport(
	/// The `<Secrets>` declaration the export is written to.
	#[field(default = Entity::PLACEHOLDER)]
	document: Entity,
	/// Write a dated series beside the declared path rather than
	/// overwriting it.
	#[field]
	dated: bool,
	/// The group the records are sealed in, whose recipients are copied from
	/// the entry document's group of the same name.
	#[field(default = SmolStr::new(SecretRecord::DEFAULT_GROUP))]
	group: SmolStr,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let store = secret_store(&cx.caller).await?;
	let target = export_target(&cx.caller, document, &store).await?;
	let entries = store.read_all().await?;
	if entries.is_empty() {
		bevybail!(
			"nothing in {} to export: the stack has not been deployed, or its \
			secrets live in another store",
			store.describe()
		);
	}
	let recipients = SecretsExport::recipients(&cx.caller, &group).await?;
	let now = Timestamp::now();
	let export = SecretsExport::document(
		target.media_type()?,
		&store,
		&group,
		recipients,
		now,
		entries,
	)?;
	let destination = match dated {
		true => target.dated(now)?,
		false => target,
	};
	if !dated
		&& destination.exists().await?
		&& SecretsExport::unchanged(&destination.read().await?, &export, &group)
	{
		info!(
			"{} secret(s) unchanged since the last export to {}, not rewritten",
			export.secrets.len(),
			destination.describe()
		);
		return Pass(cx.input).xok();
	}
	destination.write(&export).await?;
	info!(
		"{} secret(s) from {} exported to {} in {}, sealed to {} recipient(s)",
		export.secrets.len(),
		store.describe(),
		destination.describe(),
		destination.store.describe(),
		export.groups[&group].recipients.len()
	);
	Pass(cx.input).xok()
}

impl SecretsExport {
	/// The recipients an export seals to: `group`'s list in the entry's own
	/// document (the one labelled `secrets`, else the conventional file), so
	/// the humans are listed once. With no such document, or no such group in
	/// it, the discovered identity's own recipients, logged so; with no
	/// identity either, an error with the `keygen` guidance.
	pub async fn recipients(
		caller: &AsyncEntity,
		group: &str,
	) -> Result<Vec<AgeRecipient>> {
		if let Ok(entry) = SecretsHandle::resolve(caller, None).await
			&& entry.exists().await?
		{
			match entry.read().await?.groups.get(group) {
				Some(listed) => return listed.recipients.clone().xok(),
				None => warn!(
					"document {} declares no group `{group}` to copy recipients \
					from",
					entry.describe()
				),
			}
		}
		let own = AgeIdentityFile::require()?.recipients();
		if own.is_empty() {
			bevybail!(
				"the identity file holds no identity: `beet vault/keygen` makes \
				one"
			);
		}
		warn!(
			"no entry document lists group `{group}`: sealing the export to \
			this identity's {} recipient(s) alone",
			own.len()
		);
		own.xok()
	}

	/// The document an export holds: `origin` naming the stack and the
	/// store, and one record per label in `group`, sealed once.
	pub fn document(
		media_type: MediaType,
		store: &SecretStore,
		group: &str,
		recipients: Vec<AgeRecipient>,
		exported: Timestamp,
		entries: Vec<(SecretEntry, String)>,
	) -> Result<SecretsDocument> {
		let mut document = SecretsDocument::new(media_type);
		document.origin = Some(SecretsOrigin {
			app: store.stack().app_name().clone(),
			stage: store.stack().stage().clone(),
			region: store.region(),
			provider: store.id().into(),
			exported,
		});
		document.seal_records(
			group,
			recipients,
			entries.into_iter().map(|(entry, value)| {
				(
					entry.secret.label().clone(),
					SmolStr::new(value),
					SecretRecord {
						note: entry.note,
						modified: entry.modified,
						address: Some(entry.address),
						rotation: entry.rotation,
						..default()
					},
				)
			}),
		)?;
		document.xok()
	}

	/// Whether `existing` already holds what `next` would write: the same
	/// records with the same metadata, the same recipients in `group` and the
	/// same origin, `exported` aside. A record's `modified` moves with its
	/// value on every provider, so an unchanged index is an unchanged store.
	pub fn unchanged(
		existing: &SecretsDocument,
		next: &SecretsDocument,
		group: &str,
	) -> bool {
		let same_origin = match (&existing.origin, &next.origin) {
			(Some(existing), Some(next)) => {
				SecretsOrigin {
					exported: next.exported,
					..existing.clone()
				} == *next
			}
			(None, None) => true,
			_ => false,
		};
		let same_recipients =
			match (existing.groups.get(group), next.groups.get(group)) {
				(Some(existing), Some(next)) => {
					existing.recipients.iter().collect::<HashSet<_>>()
						== next.recipients.iter().collect::<HashSet<_>>()
				}
				_ => false,
			};
		same_origin && same_recipients && existing.secrets == next.secrets
	}
}

/// The secret store of the stack `caller` sits under.
pub(crate) async fn secret_store(caller: &AsyncEntity) -> Result<SecretStore> {
	caller
		.with_state::<StackQuery, _>(|entity, stacks| {
			stacks.secret_store(entity)
		})
		.await?
}

/// The document `declaration` names, in the store it is written through.
///
/// A declaration targeting an [`R2BucketBlock`] (`{StoreRef($cold)}`) is
/// reached under the pair the apply parked rather than the process's own
/// credentials, which are the other vendor's: the runtime
/// attach lands a store under the ambient credentials at declaration time,
/// synchronously and on every target, while the pair is an async read of the
/// secret store on the deploy machine alone, so the credentialed store is
/// resolved here, by the verb that writes, rather than at attach.
pub(crate) async fn export_target(
	caller: &AsyncEntity,
	declaration: Entity,
	store: &SecretStore,
) -> Result<SecretsHandle> {
	if declaration == Entity::PLACEHOLDER {
		bevybail!(
			"no document to export to: name a `<Secrets>` declaration, ie \
			`<SecretsExport document={{$repo_export}}/>`"
		);
	}
	let entity = caller.world().entity(declaration);
	let handle = SecretsHandle::of_declaration(&entity).await?;
	#[cfg(all(feature = "cloudflare_dns", feature = "aws_sdk"))]
	if let Some(block) = entity
		.world()
		.with(move |world: &mut World| -> Option<R2BucketBlock> {
			let target = world.get::<StoreRef>(declaration)?.store();
			world.get::<R2BucketBlock>(target).cloned()
		})
		.await
	{
		let cold = block.parked_store(store).await?;
		return SecretsHandle::new(cold, handle.path.as_str())?
			.with_label(handle.label.clone().unwrap_or_default())
			.xok();
	}
	#[cfg(not(all(feature = "cloudflare_dns", feature = "aws_sdk")))]
	let _ = store;
	handle.xok()
}

#[cfg(test)]
pub(crate) mod tests {
	use super::*;
	use crate::types::test_support::*;

	/// A stack with a document store, two secrets in it, and an export
	/// declaration in the repo store.
	pub(crate) async fn exported_stack(
		world: &mut World,
	) -> (Entity, Entity, SecretStore) {
		let export = world.spawn_empty().id();
		let root = world
			.spawn((
				Stack::new("mail").with_stage("prod"),
				BlobStore::temp(),
				RepoStore,
				children![DocumentSecrets::new("store.toml")],
			))
			.id();
		world.entity_mut(export).insert((
			Secrets::new("infra/secrets/mail--prod.toml")
				.with_label("mail-prod"),
			ChildOf(root),
		));
		world.flush();
		let store = world
			.with_state::<StackQuery, _>(|stacks| stacks.secret_store(root))
			.unwrap();
		store
			.create(
				&SecretRef::new("dkim-example-com"),
				"-----BEGIN PRIVATE KEY-----",
				Some("the signing key"),
				SecretRotation::manual("a new selector"),
			)
			.await
			.unwrap();
		store
			.create(
				&SecretRef::new("mail-tlsa"),
				"abc",
				None,
				SecretRotation::Remint,
			)
			.await
			.unwrap();
		(root, export, store)
	}

	/// Run one export step under `root`.
	pub(crate) async fn run_export(
		world: &mut World,
		root: Entity,
		document: Entity,
		dated: bool,
	) -> Result {
		let outcome = world
			.spawn((
				SecretsExport {
					document,
					dated,
					..default()
				},
				ChildOf(root),
			))
			.run_async_then(|entity| async move {
				entity
					.call::<Request, Outcome<Request, Response>>(Request::get(
						"",
					))
					.await
			})
			.await?;
		match outcome {
			Outcome::Pass(_) => Ok(()),
			Outcome::Fail(response) => {
				bevybail!("failed: {}", response.status())
			}
		}
	}

	/// The document a stack exports: its origin, every label with its note
	/// and address, sealed to the entry document's `default` list, readable
	/// by that list and not by the exporter alone. Snapshotted with the
	/// per-run recipients and blob cut.
	#[beet_core::test]
	async fn exports_the_store_into_a_document() {
		let mut world = infra_world();
		let (root, export, _) = exported_stack(&mut world).await;
		// the declared entry document lists alice in `default`
		let alice = AgeIdentity::generate();
		let mut alice_file = AgeIdentityFile::default();
		alice_file.push(alice.clone());
		world.spawn((Secrets::default(), ChildOf(root)));
		world.flush();
		let repo = world.get::<BlobStore>(root).unwrap().clone();
		let entry = SecretsHandle::new(repo.clone(), "secrets.toml").unwrap();
		let mut document = SecretsDocument::default();
		document.set(&alice_file, "A", "1", default()).unwrap();
		entry.write(&document).await.unwrap();

		run_export(&mut world, root, export, false).await.unwrap();
		let written =
			SecretsHandle::new(repo.clone(), "infra/secrets/mail--prod.toml")
				.unwrap();
		let exported = written.read().await.unwrap();
		let origin = exported.origin.clone().unwrap();
		origin.app.as_str().xpect_eq("mail");
		origin.stage.as_str().xpect_eq("prod");
		origin.provider.as_str().xpect_eq("document");
		exported.groups["default"]
			.recipients
			.xpect_eq(vec![alice.to_recipient()]);
		let opened = exported.open(&alice_file).unwrap();
		let key = opened.get("dkim-example-com").unwrap();
		key.value.as_str().xpect_eq("-----BEGIN PRIVATE KEY-----");
		key.record
			.note
			.clone()
			.unwrap()
			.as_str()
			.xpect_eq("the signing key");
		key.record
			.address
			.clone()
			.unwrap()
			.as_str()
			.xpect_eq("store.toml#dkim-example-com");
		key.record.modified.xpect_some();
		opened
			.get("mail-tlsa")
			.unwrap()
			.value
			.as_str()
			.xpect_eq("abc");
		// the index reads without a key; the blob and lists differ per run
		let text = String::from_utf8(exported.to_bytes().unwrap()).unwrap();
		let mut in_blob = false;
		text.lines()
			.filter(|line| {
				let armor = line.starts_with("-----");
				in_blob = (in_blob || armor) && !(in_blob && armor);
				!in_blob
					&& !armor && !line.starts_with("recipients")
					&& !line.starts_with("exported")
					&& !line.starts_with("modified")
			})
			.collect::<Vec<_>>()
			.join("\n")
			.xpect_snapshot();
	}

	/// An export whose store has not changed is not rewritten, and one whose
	/// store has is.
	#[beet_core::test]
	async fn an_unchanged_export_is_not_rewritten() {
		let mut world = infra_world();
		let (root, export, store) = exported_stack(&mut world).await;
		let repo = world.get::<BlobStore>(root).unwrap().clone();
		let written =
			SecretsHandle::new(repo, "infra/secrets/mail--prod.toml").unwrap();
		run_export(&mut world, root, export, false).await.unwrap();
		let first = written.read().await.unwrap();
		run_export(&mut world, root, export, false).await.unwrap();
		written.read().await.unwrap().xpect_eq(first.clone());
		store
			.overwrite(&SecretRef::new("mail-tlsa"), "def", None, None)
			.await
			.unwrap();
		run_export(&mut world, root, export, false).await.unwrap();
		let second = written.read().await.unwrap();
		(second != first).xpect_true();
		SecretsExport::unchanged(&first, &second, "default").xpect_false();
		SecretsExport::unchanged(&second, &second, "default").xpect_true();
	}

	/// A dated export lands beside the declared path in the series shape,
	/// and the declared path itself is never written.
	#[beet_core::test]
	async fn dated_exports_form_a_series() {
		let mut world = infra_world();
		let (root, export, _) = exported_stack(&mut world).await;
		let repo = world.get::<BlobStore>(root).unwrap().clone();
		run_export(&mut world, root, export, true).await.unwrap();
		let declared =
			SecretsHandle::new(repo, "infra/secrets/mail--prod.toml").unwrap();
		declared.exists().await.unwrap().xpect_false();
		let newest = declared.newest_dated().await.unwrap().unwrap();
		SecretsHandle::is_dated(
			newest.path.as_str().strip_prefix("infra/secrets/").unwrap(),
			"toml",
		)
		.xpect_true();
		newest.read().await.unwrap().secrets.len().xpect_eq(2);
	}

	/// An empty store and an unnamed document both refuse.
	#[beet_core::test]
	async fn refuses_an_empty_store_and_no_document() {
		let mut world = infra_world();
		let root = world
			.spawn((
				Stack::new("mail").with_stage("dev"),
				BlobStore::temp(),
				RepoStore,
				children![DocumentSecrets::new("store.toml")],
			))
			.id();
		let export = world.spawn((Secrets::new("x.toml"), ChildOf(root))).id();
		world.flush();
		run_export(&mut world, root, export, false)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("nothing in");
		run_export(&mut world, root, Entity::PLACEHOLDER, false)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("no document to export to");
	}
}
