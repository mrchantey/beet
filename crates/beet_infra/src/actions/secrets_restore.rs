//! A secrets document written back into a stack's secret store.
use crate::actions::export_target;
use crate::actions::secret_store;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use core::fmt::Write;

/// Request params for [`SecretsRestore`], surfaced in `--help`.
#[derive(Reflect)]
struct RestoreParams {
	/// The document to restore from: a declared `<Secrets>` label, or the
	/// path or store uri of an undeclared file. Defaults to the one declared
	/// document, an error naming the labels when several are.
	document: Option<String>,
	/// One document of a dated series, as its key relative to the series
	/// dir (`2026/09/15/010101Z.toml`); absent, the newest when the declared
	/// path itself is not written.
	from: Option<String>,
	/// Restore only these labels, comma separated; absent, every record.
	only: Option<String>,
	/// Overwrite secrets the store already holds. Without it a single
	/// existing label refuses the whole restore, since a restore that
	/// silently replaced a live credential is a rotation nobody asked for.
	force: bool,
}

/// `<SecretsRestore/>` — write every record of a secrets document into the
/// stack's secret store by label, the inverse of [`SecretsExport`] and how a
/// fresh account or a rebuilt stack starts: each record's value and note
/// land through `overwrite` at the address the stack's store composes, so a
/// document exported from one provider or region restores into another.
///
/// Refuses without `--force` when the store already holds any of the labels,
/// naming them. Logs every label and never a value.
///
/// ```sh
/// beet mail/secrets/restore --document=mail-prod --stage=prod
/// beet mail/secrets/restore --document=mail-cold --from=2026/09/15/010101Z.toml
/// beet mail/secrets/restore --document=mail-prod --only=dkim-beetmash-com --force
/// ```
#[action]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(ParamsPartial = ParamsPartial::new::<RestoreParams>())]
pub async fn SecretsRestore(cx: ActionContext<Request>) -> Result<Response> {
	let params = cx.input.parse_params::<RestoreParams>()?;
	let store = secret_store(&cx.caller).await?;
	let source = SecretsRestore::source(
		&cx.caller,
		params.document.as_deref(),
		params.from.as_deref(),
		&store,
	)
	.await?;
	let document = source.read().await?;
	let opened = document.open(&AgeIdentityFile::require()?)?;
	let only = params
		.only
		.as_deref()
		.map(|only| {
			only.split(',')
				.map(str::trim)
				.filter(|label| !label.is_empty())
				.map(SmolStr::new)
				.collect::<Vec<_>>()
		})
		.unwrap_or_default();
	let records = SecretsRestore::records(&document, &opened, &only)?;
	if records.is_empty() {
		bevybail!("document {} holds no record to restore", source.describe());
	}
	// refuse to replace what is live unless told to
	let held = store
		.list()
		.await?
		.into_iter()
		.map(|entry| entry.secret.label().clone())
		.filter(|label| records.iter().any(|(name, _)| name == label))
		.collect::<Vec<_>>();
	if !held.is_empty() && !params.force {
		bevybail!(
			"{} already holds {} of the {} record(s) in {} ({}): pass --force \
			to overwrite them",
			store.describe(),
			held.len(),
			records.len(),
			source.describe(),
			held.join(", ")
		);
	}
	let mut out = String::new();
	for (name, secret) in &records {
		let secret_ref = SecretRef::new(name.clone());
		store
			.overwrite(
				&secret_ref,
				&secret.value,
				secret.record.note.as_deref(),
				secret.record.rotation.clone(),
			)
			.await?;
		writeln!(out, "restored `{name}` to {}", store.address(&secret_ref))?;
		info!("restored secret {name}");
	}
	writeln!(
		out,
		"{} record(s) restored from {} into {}",
		records.len(),
		source.describe(),
		store.describe()
	)?;
	Response::ok_text(out).xok()
}

impl SecretsRestore {
	/// The document to restore from: the named one when it is written,
	/// else `from` within its dated series, else the newest of the series.
	async fn source(
		caller: &AsyncEntity,
		selector: Option<&str>,
		from: Option<&str>,
		store: &SecretStore,
	) -> Result<SecretsHandle> {
		let declared = SecretsHandle::resolve(caller, selector).await?;
		// a declaration may target a bucket reached under a parked pair
		let label = declared_label(selector);
		let declaration = caller
			.with_state::<Query<(Entity, &Secrets)>, _>(move |_, declared| {
				declared
					.iter()
					.find(|(_, secrets)| secrets.label == label)
					.map(|(entity, _)| entity)
			})
			.await?;
		let handle = match declaration {
			Some(declaration) => {
				export_target(caller, declaration, store).await?
			}
			None => declared,
		};
		if let Some(from) = from {
			let dir = handle
				.path
				.as_str()
				.rsplit_once('/')
				.map(|(dir, _)| format!("{dir}/"))
				.unwrap_or_default();
			return SecretsHandle::new(
				handle.store.clone(),
				format!("{dir}{from}"),
			)?
			.with_label(handle.label.clone().unwrap_or_default())
			.xok();
		}
		if handle.exists().await? {
			return handle.xok();
		}
		handle.newest_dated().await?.ok_or_else(|| {
			bevyhow!(
				"document {} is not written and no dated export sits beside it",
				handle.describe()
			)
		})
	}

	/// The records to restore: every opened one, narrowed to `only`, each
	/// of which must be in the document and opened.
	fn records<'a>(
		document: &SecretsDocument,
		opened: &'a OpenSecrets,
		only: &[SmolStr],
	) -> Result<Vec<(SmolStr, &'a Secret)>> {
		if !only.is_empty() {
			return only
				.iter()
				.map(|label| {
					opened
						.get(label)
						.map(|secret| (label.clone(), secret))
						.ok_or_else(|| {
							match document.secrets.contains_key(label) {
								true => bevyhow!(
									"record `{label}` is in a group this identity \
								cannot open"
								),
								false => bevyhow!(
									"no record `{label}` in the document"
								),
							}
						})
				})
				.collect();
		}
		let locked = document
			.secrets
			.keys()
			.filter(|name| opened.get(name).is_none())
			.cloned()
			.collect::<Vec<_>>();
		if !locked.is_empty() {
			bevybail!(
				"this identity cannot open the group(s) holding {}: a restore \
				writes every record or none (`--only` narrows it)",
				locked.join(", ")
			);
		}
		opened
			.secrets
			.iter()
			.map(|(name, secret)| (name.clone(), secret))
			.collect::<Vec<_>>()
			.xok()
	}
}

/// The label a `--document` selector names, the default when none.
fn declared_label(selector: Option<&str>) -> SmolStr {
	match DocumentSelector::parse(selector) {
		DocumentSelector::Label(label) => label,
		_ => Secrets::DEFAULT_LABEL.into(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::actions::secrets_export::tests::*;
	use crate::types::test_support::*;
	use beet_action::prelude::*;

	/// Run one restore under `root` with `args`.
	async fn restore(
		world: &mut World,
		root: Entity,
		args: &str,
	) -> Result<String> {
		let request = Request::from_cli_str(args);
		world
			.spawn((SecretsRestore, ChildOf(root)))
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

	/// Export, wipe the store, restore: the store holds exactly what it
	/// held, notes included; a second restore refuses without `--force` and
	/// `--only` narrows it.
	#[beet_core::test]
	async fn restore_is_the_inverse_of_export() {
		let mut world = infra_world();
		let (root, export, store) = exported_stack(&mut world).await;
		let before = store.read_all().await.unwrap();
		run_export(&mut world, root, export, false).await.unwrap();
		// wipe
		let labels = before
			.iter()
			.map(|(entry, _)| entry.secret.clone())
			.collect::<Vec<_>>();
		store.delete(&labels).await.unwrap();
		store.list().await.unwrap().len().xpect_eq(0);

		restore(&mut world, root, "--document=mail-prod")
			.await
			.unwrap()
			.xpect_contains("restored `dkim-example-com`")
			.xpect_contains("2 record(s) restored");
		let after = store.read_all().await.unwrap();
		after.len().xpect_eq(2);
		for (entry, value) in &before {
			let restored = after
				.iter()
				.find(|(restored, _)| restored.secret == entry.secret)
				.unwrap();
			restored.1.xpect_eq(value.clone());
			restored.0.note.xpect_eq(entry.note.clone());
		}

		// live secrets refuse
		restore(&mut world, root, "--document=mail-prod")
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("already holds 2 of the 2")
			.xpect_contains("--force");
		restore(
			&mut world,
			root,
			"--document=mail-prod --only=mail-tlsa --force",
		)
		.await
		.unwrap()
		.xpect_contains("1 record(s) restored");
		restore(&mut world, root, "--document=mail-prod --only=nope --force")
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("no record `nope`");
	}

	/// With the declared path unwritten the newest dated export is the
	/// source, and `--from` names another.
	#[beet_core::test]
	async fn restores_from_a_dated_series() {
		let mut world = infra_world();
		let (root, export, store) = exported_stack(&mut world).await;
		run_export(&mut world, root, export, true).await.unwrap();
		store.delete(&[SecretRef::new("mail-tlsa")]).await.unwrap();
		restore(&mut world, root, "--document=mail-prod --only=mail-tlsa")
			.await
			.unwrap()
			.xpect_contains("restored `mail-tlsa`");
		restore(
			&mut world,
			root,
			"--document=mail-prod --from=1999/01/01/000000Z.toml --force",
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("not written yet");
	}
}
