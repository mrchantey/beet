//! Removing a human from a stack's secrets: re-seal what they could read,
//! rotate what they could have read.
use crate::actions::stack_and_store;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;
use core::fmt::Write;

/// Request params for [`SecretsRevoke`], surfaced in `--help`.
#[derive(Reflect)]
struct RevokeParams {
	/// The recipient (`age1..`) being removed. It must already be out of
	/// every group's list in every declared document: the human edits the
	/// lists first, and the verb refuses while they are still listed.
	recipient: Option<String>,
	/// Print the ledger of what would be re-sealed and rotated and change
	/// nothing.
	dry_run: bool,
}

/// `<SecretsRevoke/>` — remove a human from a stack: re-seal every group of
/// every declared document to its current list, then rotate every secret in
/// the stack's store through its declared [`Rotation`], and print what needs
/// a hand.
///
/// Git history and the cold bucket keep old ciphertext a removed human can
/// still read, so removal means re-sealing every group they were in AND
/// rotating every secret those groups held. The recipient must already be
/// out of the lists (the verb refuses while they are listed, since a rekey
/// to a list that still names them changes nothing). Then every secret of
/// the store rotates by what minted it declared: a [`Rotation::Replace`]
/// through `tofu apply -replace` on its resource, a [`Rotation::Remint`] by
/// deleting the entry and running the stack's `deploy` group, which mints a
/// fresh one and re-provisions its consumer, and a [`Rotation::Manual`] is
/// printed with its reason as the residue, as is a secret with no rotation
/// declared (one parked by hand). It never reports a rotation it did not
/// perform, and ends with the count of each.
///
/// ```sh
/// beet mail/secrets/revoke --recipient=age1.. --stage=prod --dry-run
/// beet mail/secrets/revoke --recipient=age1.. --stage=prod
/// ```
#[action]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(ParamsPartial = ParamsPartial::new::<RevokeParams>())]
pub async fn SecretsRevoke(cx: ActionContext<Request>) -> Result<Response> {
	let params = cx.input.parse_params::<RevokeParams>()?;
	let recipient = params
		.recipient
		.as_deref()
		.ok_or_else(|| {
			bevyhow!(
				"name the recipient being removed, ie `--recipient=age1..`"
			)
		})?
		.parse::<AgeRecipient>()?;
	let mut ledger = String::new();
	let dry = params.dry_run;
	if dry {
		writeln!(ledger, "dry run: nothing below is changed")?;
	}

	// the documents: refused while the recipient is listed, re-sealed else
	let documents = SecretsHandle::declared(&cx.caller)
		.await?
		.into_iter()
		.map(|(label, handle)| {
			handle.map_err(|err| bevyhow!("document `{label}`: {err}"))
		})
		.collect::<Result<Vec<_>>>()?;
	let mut opened = Vec::new();
	for handle in &documents {
		if !handle.exists().await? {
			continue;
		}
		let document = handle.read().await?;
		if let Some((group, _)) = document
			.groups
			.iter()
			.find(|(_, group)| group.recipients.contains(&recipient))
		{
			bevybail!(
				"{recipient} is still listed in group `{group}` of document {}: \
				remove it from every list first, then revoke",
				handle.describe()
			);
		}
		opened.push((handle, document));
	}
	let identities = AgeIdentityFile::require()?;
	for (handle, mut document) in opened {
		let report = match dry {
			true => RekeyReport {
				rekeyed: document
					.groups
					.iter()
					.filter(|(_, group)| group.sealed.is_some())
					.map(|(name, _)| name.clone())
					.filter(|name| document.is_member(name, &identities))
					.collect(),
				locked: Vec::new(),
			},
			false => {
				let report = document.rekey(&identities)?;
				handle.write(&document).await?;
				report
			}
		};
		writeln!(
			ledger,
			"document {}: {} group(s) re-sealed{}{}",
			handle.describe(),
			report.rekeyed.len(),
			match report.rekeyed.is_empty() {
				true => String::new(),
				false => format!(" ({})", report.rekeyed.join(", ")),
			},
			match report.locked.is_empty() {
				true => String::new(),
				false => format!(
					"; not a member of {}, a member re-seals those",
					report.locked.join(", ")
				),
			}
		)?;
	}

	// the store: every secret by its declared rotation
	let (stack, store) = stack_and_store(&cx.caller).await?;
	let plan = RevokePlan::new(store.list(&stack).await?);
	for (secret, resource) in &plan.replace {
		writeln!(ledger, "replace `{secret}`: tofu apply -replace={resource}")?;
	}
	for secret in &plan.remint {
		writeln!(
			ledger,
			"remint `{secret}`: deleted, the next deploy mints it"
		)?;
	}
	for (secret, why) in &plan.manual {
		writeln!(ledger, "manual `{secret}`: {why}")?;
	}
	for secret in &plan.undeclared {
		writeln!(
			ledger,
			"manual `{secret}`: no rotation declared (parked by hand), rotate \
			it where it was made"
		)?;
	}
	if !dry {
		if !plan.replace.is_empty() {
			let project = terra::Project::resolve(&cx.caller).await?;
			let resources = plan.resources();
			info!(
				"replacing {} resource(s): {}",
				resources.len(),
				resources.join(", ")
			);
			project.apply_replacing(&resources).await?;
		}
		if !plan.remint.is_empty() {
			let labels = plan
				.remint
				.iter()
				.map(|label| SecretRef::new(label.clone()))
				.collect::<Vec<_>>();
			let deleted = store.delete(&stack, &labels).await?;
			for secret in &deleted {
				info!("deleted secret {} for re-minting", secret.label());
			}
			run_deploy(&cx.caller).await?;
		}
	}
	writeln!(
		ledger,
		"{}: {} replaced, {} re-minted, {} manual",
		match dry {
			true => "would rotate",
			false => "rotated",
		},
		plan.replace.len(),
		plan.remint.len(),
		plan.manual.len() + plan.undeclared.len()
	)?;
	Response::ok_text(ledger).xok()
}

/// A store's secrets sorted by how each rotates.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RevokePlan {
	/// Label and the terraform resource to replace.
	pub replace: Vec<(SmolStr, SmolStr)>,
	/// Labels to delete ahead of a deploy.
	pub remint: Vec<SmolStr>,
	/// Label and why a hand rotates it.
	pub manual: Vec<(SmolStr, SmolStr)>,
	/// Labels with no rotation stored, parked by hand.
	pub undeclared: Vec<SmolStr>,
}

impl RevokePlan {
	/// Sort `entries` by rotation.
	pub fn new(entries: Vec<SecretEntry>) -> Self {
		let mut plan = Self::default();
		for entry in entries {
			let label = entry.secret.label().clone();
			match entry.rotation {
				Some(Rotation::Replace { resource }) => {
					plan.replace.push((label, resource))
				}
				Some(Rotation::Remint) => plan.remint.push(label),
				Some(Rotation::Manual { why }) => {
					plan.manual.push((label, why))
				}
				None => plan.undeclared.push(label),
			}
		}
		plan
	}

	/// The distinct resources to replace, in first-seen order: two secrets
	/// derived from one resource are one replacement.
	pub fn resources(&self) -> Vec<String> {
		let mut resources = Vec::<String>::new();
		for (_, resource) in &self.replace {
			if !resources.iter().any(|seen| seen == resource.as_str()) {
				resources.push(resource.to_string());
			}
		}
		resources
	}
}

/// Run the stack's `deploy` verb (the forward group `<DeployRoutes/>` names),
/// which mints every re-minted secret and re-provisions its consumer.
async fn run_deploy(caller: &AsyncEntity) -> Result {
	let deploy = caller
		.with_state::<(
			StackQuery,
			Query<
				(Entity, &PathPartial, &RunGroup<Request, Response>),
				With<ExchangeGroup>,
			>,
		), _>(|entity, (stacks, routes)| {
			let declared = stacks.declared(entity)?;
			routes
				.iter()
				.filter(|(entity, _, _)| declared.contains(entity))
				.find(|(_, path, group)| {
					!group.reverse
						&& path.segments.last().is_some_and(|segment| {
							segment.to_string() == "deploy"
						})
				})
				.map(|(entity, _, _)| entity)
				.ok_or_else(|| {
					bevyhow!(
						"no `deploy` route under this stack to re-mint through: \
						author `<DeployRoutes deploy={{$up}} destroy={{$down}}/>`"
					)
				})
		})
		.await??;
	info!("running the stack's deploy to re-mint");
	let response = caller
		.world()
		.entity(deploy)
		.call::<Request, Response>(Request::get("/deploy"))
		.await?;
	match response.status().is_ok() {
		true => Ok(()),
		false => bevybail!(
			"the re-minting deploy failed with {}: {}",
			response.status(),
			response.text().await.unwrap_or_default()
		),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::types::test_support::*;

	/// A stack with a document store holding one secret of each rotation,
	/// an entry document listing the process identity and alice in
	/// `default`, an empty deploy group and the verb routes.
	async fn revocable_stack(
		world: &mut World,
	) -> (
		Entity,
		SecretStore,
		ResolvedStack,
		AgeIdentity,
		SecretsHandle,
	) {
		let alice = AgeIdentity::generate();
		let deploy = world.spawn(Group).id();
		let destroy = world.spawn(Group).id();
		let root = world
			.spawn((
				Stack::new("mail").with_stage("prod"),
				BlobStore::temp(),
				RepoStore,
				children![
					DocumentSecrets::new("store.toml"),
					Secrets::default(),
					Router::with_defaults()
				],
			))
			.id();
		world.flush();
		let router = world.entity(root).get::<Children>().unwrap()[2];
		world
			.entity_mut(router)
			.insert_template(DeployRoutes {
				deploy: Some(deploy),
				destroy: Some(destroy),
			})
			.unwrap();
		world.flush();
		let (stack, store) = world
			.with_state::<StackQuery, _>(|stacks| {
				stacks
					.secret_store(root)
					.map(|store| (stacks.resolve(root), store))
			})
			.unwrap();
		for (label, rotation) in [
			(
				"cold-access-key-id",
				Rotation::replace("cloudflare_account_token.x"),
			),
			(
				"cold-secret-access-key",
				Rotation::replace("cloudflare_account_token.x"),
			),
			("mail-admin-password", Rotation::Remint),
			("dkim-example-com", Rotation::manual("a new selector")),
		] {
			store
				.create(&stack, &SecretRef::new(label), "x", None, rotation)
				.await
				.unwrap();
		}
		// one parked by hand, with no rotation
		store
			.overwrite(
				&stack,
				&SecretRef::new("comail-did"),
				"did:plc",
				None,
				None,
			)
			.await
			.unwrap();
		// the entry document, sealed to both humans
		let entry = SecretsHandle::new(
			world.get::<BlobStore>(root).unwrap().clone(),
			"secrets.toml",
		)
		.unwrap();
		let mut identities = AgeIdentityFile::default();
		identities.push(process_identity());
		let mut document = SecretsDocument::default();
		document.groups.insert(
			"default".into(),
			SecretsGroup::new(vec![
				process_identity().to_recipient(),
				alice.to_recipient(),
			]),
		);
		document.set(&identities, "TOKEN", "1", default()).unwrap();
		entry.write(&document).await.unwrap();
		(root, store, stack, alice, entry)
	}

	async fn revoke(
		world: &mut World,
		root: Entity,
		args: &str,
	) -> Result<String> {
		let request = Request::from_cli_str(args);
		world
			.spawn((SecretsRevoke, ChildOf(root)))
			.run_async_then(|entity| async move {
				entity.call::<Request, Response>(request).await
			})
			.await?
			.into_result()
			.await?
			.unwrap_str()
			.await
			.xok()
	}

	/// Refused while the recipient is listed; a dry run then prints the
	/// whole ledger and changes nothing; the real run re-seals so alice can
	/// no longer read, deletes the re-mintable secret and runs the deploy.
	#[beet_core::test]
	async fn revokes_a_recipient() {
		let mut world = infra_world();
		let (root, store, stack, alice, entry) =
			revocable_stack(&mut world).await;
		let alice_recipient = alice.to_recipient().to_string();
		let args = format!("--recipient={alice_recipient}");
		revoke(&mut world, root, &args)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("still listed in group `default`");

		// the human takes alice out of the list
		let mut document = entry.read().await.unwrap();
		document
			.groups
			.get_mut("default")
			.unwrap()
			.recipients
			.retain(|recipient| recipient != &alice.to_recipient());
		entry.write(&document).await.unwrap();
		let mut alice_file = AgeIdentityFile::default();
		alice_file.push(alice.clone());
		entry
			.read()
			.await
			.unwrap()
			.open(&alice_file)
			.unwrap()
			.get("TOKEN")
			.xpect_some();

		let ledger = revoke(&mut world, root, &format!("{args} --dry-run"))
			.await
			.unwrap();
		ledger
			.as_str()
			.xpect_contains("dry run")
			.xpect_contains("document `secrets` (secrets.toml): 1 group(s) re-sealed (default)")
			.xpect_contains("replace `cold-access-key-id`: tofu apply -replace=cloudflare_account_token.x")
			.xpect_contains("remint `mail-admin-password`")
			.xpect_contains("manual `dkim-example-com`: a new selector")
			.xpect_contains("manual `comail-did`: no rotation declared")
			.xpect_contains("would rotate: 2 replaced, 1 re-minted, 2 manual");
		// nothing changed
		entry
			.read()
			.await
			.unwrap()
			.open(&alice_file)
			.unwrap()
			.get("TOKEN")
			.xpect_some();
		store.list(&stack).await.unwrap().len().xpect_eq(5);

		// the store here has a `replace`, which needs tofu: dry runs only
		// for it, so the live run is against the rest
		store
			.delete(&stack, &[
				SecretRef::new("cold-access-key-id"),
				SecretRef::new("cold-secret-access-key"),
			])
			.await
			.unwrap();
		let ledger = revoke(&mut world, root, &args).await.unwrap();
		ledger
			.as_str()
			.xpect_contains("rotated: 0 replaced, 1 re-minted, 2 manual");
		entry
			.read()
			.await
			.unwrap()
			.open(&alice_file)
			.unwrap()
			.get("TOKEN")
			.xpect_none();
		store
			.get(&stack, &SecretRef::new("mail-admin-password"))
			.await
			.unwrap()
			.xpect_none();
		store
			.get(&stack, &SecretRef::new("dkim-example-com"))
			.await
			.unwrap()
			.xpect_some();
	}

	/// Two secrets derived from one resource are one replacement.
	#[beet_core::test]
	fn a_plan_sorts_by_rotation() {
		let entry = |label: &str, rotation: Option<Rotation>| SecretEntry {
			secret: SecretRef::new(label),
			address: label.into(),
			note: None,
			modified: None,
			rotation,
		};
		let plan = RevokePlan::new(vec![
			entry("a", Some(Rotation::replace("r.x"))),
			entry("b", Some(Rotation::replace("r.x"))),
			entry("c", Some(Rotation::replace("r.y"))),
			entry("d", Some(Rotation::Remint)),
			entry("e", Some(Rotation::manual("why"))),
			entry("f", None),
		]);
		plan.resources()
			.xpect_eq(vec!["r.x".to_string(), "r.y".to_string()]);
		plan.remint.xpect_eq(vec![SmolStr::new("d")]);
		plan.manual.xpect_eq(vec![("e".into(), "why".into())]);
		plan.undeclared.xpect_eq(vec![SmolStr::new("f")]);
	}
}
