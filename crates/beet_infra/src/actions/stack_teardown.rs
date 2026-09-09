//! Removing what carries a stack's state, once its resources are gone.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::BlobStoreProvider;
use beet_net::prelude::*;

/// The teardown step that removes a stack's state carriers: the tofu state
/// object, the native S3 lock file beside it, the artifacts bucket, and the
/// working directory.
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
/// One asymmetry is deliberate and worth naming: the artifacts bucket's *ensure*
/// stays fused inside [`TofuApply`], which creates it and uploads into it in one
/// step, while its *removal* lives here. Splitting the apply to move the ensure
/// into the deploy group would separate the bucket from the upload that is the
/// only reason it exists.
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
	let [state, lock, artifacts, work_dir] = StackTeardown::CARRIERS;

	// the tofu state object
	report(state, backend.provider().remove(&state_path).await);
	// the native S3 lock beside it, left by an interrupted run
	report(
		lock,
		backend
			.provider()
			.remove(&SmolPath::new(format!("{state_path}.tflock")))
			.await,
	);
	// the artifacts bucket, which terraform does not own
	report(artifacts, project.artifacts().store().store_remove().await);
	// the rendered config, lockfile and scratch files
	report(work_dir, fs_ext::remove_async(&project.work_dir()).await);

	Pass(cx.input).xok()
}

impl StackTeardown {
	/// Everything a stack carries its state in, and the whole of what this
	/// action removes.
	///
	/// A list rather than four unrelated calls because it IS an inventory: these
	/// are the four the driver used to sweep in a hardcoded `destroy_common` no
	/// block could add to, and the point of moving them here is that the set is
	/// now visible and orderable rather than buried.
	pub const CARRIERS: [&'static str; 4] = [
		"state object",
		"state lock",
		"artifacts bucket",
		"work directory",
	];
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

	/// The inventory this action took over from the driver's hardcoded sweep.
	#[beet_core::test]
	fn the_carriers_are_the_ones_the_driver_used_to_sweep() {
		StackTeardown::CARRIERS.xpect_eq([
			"state object",
			"state lock",
			"artifacts bucket",
			"work directory",
		]);
	}
}
