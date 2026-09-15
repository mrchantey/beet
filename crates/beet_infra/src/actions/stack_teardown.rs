//! Removing what carries a stack's state, once its resources are gone.
use crate::actions::ssm_ext;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The teardown step that removes a stack's state carriers: the tofu state
/// object, the native S3 lock file beside it, the working directory, and every
/// secret the stack's actions minted under its parameter store prefix.
///
/// These are the things a deploy needs in place BEFORE terraform runs, so under
/// the one rule — teardown order is convergence order reversed — they come off
/// last, after [`TofuDestroy`]. A destroy group therefore declares this first:
///
/// ```bsx
/// <Group bx:ref="down">
///     <StackTeardown/>
///     <TofuDestroy/>
/// </Group>
/// ```
///
/// Each removal is best-effort and logged: these are the carriers themselves, so
/// a state object that has already gone must not strand the work directory, and
/// half a teardown is worse than a noisy whole one.
///
/// The secret prefix is the newest carrier and the one with teeth. Terraform's
/// own parameters go with `tofu destroy`, but the ones an action minted
/// ([`EnsureSecret`], [`EnsureDkimKey`], a mailbox credential) are unknown to
/// it and outlived every destroy, which left a restore drill's five parameters
/// to be deleted by hand each time and, on a real stack, a DKIM private key
/// and every mailbox password lying under a prefix nothing declared any more.
/// The sweep is unconditional and logs each NAME it removes, never a value. It
/// must run after the destroy rather than before it, since a destroy renders
/// the config and a content variable (the DKIM public key) reads parameter
/// store to do so; a stack whose secrets must outlive its destroy exports them
/// first, which is what the mail stack's cold copy is for.
///
/// The repo store is not here: it is terraform's, declared as a store block,
/// so [`TofuDestroy`] removes it with every other resource and `force_destroy`
/// takes the versions and release pointers with the bucket.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn StackTeardown(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let project = terra::Project::resolve(&cx.caller).await?;
	let deployment = project.deployment();
	let backend = deployment.backend();
	let state_path = deployment.backend_path(&project);
	let [state, lock, work_dir, secrets] = StackTeardown::CARRIERS;

	let store = backend.store()?;
	// the tofu state object
	report(state, store.remove(&state_path).await);
	// the native S3 lock beside it, left by an interrupted run
	report(
		lock,
		store
			.remove(&RelPath::new(format!("{state_path}.tflock")))
			.await,
	);
	// the rendered config, lockfile and scratch files
	report(work_dir, fs_ext::remove_async(&project.work_dir()).await);
	// what the actions minted outside terraform, which nothing else removes
	report(secrets, sweep_secrets(project.stack()).await);

	Pass(cx.input).xok()
}

impl StackTeardown {
	/// Everything a stack carries its state in, and the whole of what this
	/// action removes.
	///
	/// A list rather than four unrelated calls because it IS an inventory:
	/// these are what the driver used to sweep in a hardcoded `destroy_common`
	/// no block could add to, and the point of moving them here is that the
	/// set is now visible and orderable rather than buried. The order is the
	/// removal order, ie convergence reversed: the secrets are minted before
	/// anything else a deploy does, so they are the last thing to go.
	pub const CARRIERS: [&'static str; 4] =
		["state object", "state lock", "work directory", "secret prefix"];
}

/// Delete every parameter under the stack's secret prefix, naming each one.
async fn sweep_secrets(stack: &ResolvedStack) -> Result<usize> {
	let region = stack.region();
	let prefix = SecretRef::prefix(stack);
	let names = ssm_ext::list(region, &prefix).await?;
	if names.is_empty() {
		info!("no secrets under {prefix}");
		return Ok(0);
	}
	let deleted = ssm_ext::delete(region, &names).await?;
	for name in &deleted {
		info!("deleted secret {name}");
	}
	let missed = names
		.iter()
		.filter(|name| !deleted.contains(name))
		.collect::<Vec<_>>();
	match missed.is_empty() {
		true => Ok(deleted.len()),
		false => Err(bevyhow!(
			"{} of {} secrets under {prefix} were not deleted: {missed:?}",
			missed.len(),
			names.len()
		)),
	}
}

/// Log one removal's outcome, so a carrier that had already gone (or refuses to
/// go) is visible without stranding the ones after it.
fn report<T, E: core::fmt::Display>(
	what: &str,
	result: core::result::Result<T, E>,
) {
	match result {
		Ok(_) => info!("removed {what}"),
		Err(err) => warn!("could not remove {what}: {err}"),
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// The steps a `destroy` route would actually run, ie the group flattened
	/// and then reversed.
	fn teardown_order(world: &mut World, group: Entity) -> Vec<Entity> {
		let mut order = world
			.with_state::<GroupMembers, _>(|members| {
				members.flatten::<Request, Outcome<Request, Response>>(
					group,
					BypassErrors(
						ChildError::NO_ACTION | ChildError::ACTION_MISMATCH,
					),
				)
			})
			.unwrap();
		order.reverse();
		order
	}

	/// The one rule, end to end: a teardown group is authored in convergence
	/// order, so what a deploy needs FIRST comes off LAST. The state carriers
	/// converge before the apply and so are removed after `tofu destroy`; a
	/// declaration that attaches after it is removed before.
	#[beet_core::test]
	fn a_teardown_is_the_convergence_order_reversed() {
		let mut world = World::new();
		let down = world
			.spawn((Group, children![
				StackTeardown::default(),
				TofuDestroy::default(),
			]))
			.flush();
		let attach = world
			.spawn((Group, MemberOf(down), children![
				EipReverseDnsReset::default(),
				MtaStsUnpublish::default(),
			]))
			.flush();
		world.flush();
		let carriers = world.entity(down).get::<Children>().unwrap();
		let attached = world.entity(attach).get::<Children>().unwrap();
		let (teardown, tofu) = (carriers[0], carriers[1]);
		let (ptr, mta_sts) = (attached[0], attached[1]);

		teardown_order(&mut world, down).xpect_eq(vec![
			// the external resources first, while their stack still stands
			mta_sts, ptr,  // then terraform's own
			tofu, // and the state carriers last of all
			teardown,
		]);
	}

	/// The inventory this action took over from the driver's hardcoded sweep,
	/// less the artifacts bucket, which became the terraform-owned repo store,
	/// plus the secret prefix, which the driver never swept and which is why a
	/// destroyed stage's parameters used to be deleted by hand.
	#[beet_core::test]
	fn the_carriers_are_the_ones_the_driver_used_to_sweep() {
		StackTeardown::CARRIERS.xpect_eq([
			"state object",
			"state lock",
			"work directory",
			"secret prefix",
		]);
	}

	/// The prefix a teardown sweeps is exactly the one [`EnsureSecret`] mints
	/// under, composed by the same type, so a stack can neither miss its own
	/// secrets nor reach a neighbouring stage's.
	#[beet_core::test]
	fn the_sweep_is_scoped_to_the_stacks_own_prefix() {
		let stack = Stack::new("beetmash-mail")
			.with_stage("drill")
			.resolve(&PackageConfig::default());
		SecretRef::prefix(&stack).xpect_eq("/beetmash-mail/drill");
		SecretRef::new("mail-admin-password")
			.name(&stack)
			.xpect_eq("/beetmash-mail/drill/mail-admin-password");
	}
}
