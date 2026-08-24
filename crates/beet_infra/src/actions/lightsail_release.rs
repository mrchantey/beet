//! The post-apply release step for a Lightsail box.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<LightsailRelease/>` — after the apply, make the running box serve THIS
/// deploy's binary.
///
/// The counterpart to [`LightsailBlock::build_user_data`] rendering machine
/// config only. Because the box's cloud-init never names a version, terraform
/// has no reason to replace the instance on a code-only deploy, and nothing in
/// the apply moves the running process onto the new binary either. This step
/// closes that gap: it reaches the box over the management sshd and rolls the
/// unit onto the release the artifacts bucket now points at.
///
/// Declared as a sibling of the deploy's `<TofuApply/>`, so it resolves the
/// stack (for the deploy id and the tofu outputs) and the [`LightsailBlock`]
/// (for the management port and the unit name) by ancestry. It runs AFTER the
/// full apply, since the apply is what publishes the release pointer.
#[derive(Debug, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(LightsailReleaseAction)]
pub struct LightsailRelease {
	/// How long to wait at each gate: for ssh to answer, for the unit to
	/// appear, and for it to come up serving this deploy. Generous by default:
	/// a freshly replaced box is still installing Caddy and the CloudWatch
	/// agent when the deploy arrives.
	timeout: Duration,
	/// The gap between attempts, at every gate `timeout` bounds.
	poll: Duration,
}

impl Default for LightsailRelease {
	fn default() -> Self {
		Self {
			timeout: Duration::from_secs(300),
			poll: Duration::from_secs(5),
		}
	}
}

/// Rolls the deployed unit onto the current release and verifies it took.
///
/// Idempotent, and cheap when there is nothing to do: a box that was just
/// replaced already pulled this release at boot, so the script confirms and
/// returns rather than bouncing a healthy unit.
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn LightsailReleaseAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let release = cx
		.caller
		.get_cloned::<LightsailRelease>()
		.await
		.unwrap_or_default();
	let (project, block) = cx
		.caller
		.with_state::<ReleaseQuery, _>(|entity, query| query.resolve(entity))
		.await??;

	let deploy_id = project.stack().deploy_id().to_string();
	let connection =
		SshConnection::from_project(&project, block.management_ssh_port())
			.await?;
	connection
		.wait_for_ready(*release.timeout(), *release.poll())
		.await?;

	// the script is a file rather than an argv token: it carries quotes, globs
	// and newlines that would have to survive two shells to arrive intact.
	let script = block.release_script(
		project.stack(),
		&deploy_id,
		*release.timeout(),
		*release.poll(),
	);
	let local_path = project
		.stack()
		.work_directory()
		.into_abs()
		.join("release.sh");
	fs_ext::write_async(&local_path, script.as_bytes()).await?;
	// the ssh user's home dir, not /tmp: a predictable world-writable path is
	// a symlink-planting target, and root runs this script
	let remote_path = "beet-release.sh";
	connection.scp_to(local_path.as_path(), remote_path).await?;

	info!("releasing {deploy_id} on {}", connection.host);
	// `-n`: the ssh side is `BatchMode`, so a sudo that wanted a password would
	// hang the deploy on a prompt nobody can answer
	let output = connection
		.run_command(&format!("sudo -n bash ./{remote_path}"))
		.await?;
	// the script narrates on stderr and prints the release it converged on
	for line in String::from_utf8_lossy(&output.stderr).lines() {
		info!("{line}");
	}
	info!(
		"{} is serving release {}",
		connection.host,
		String::from_utf8_lossy(&output.stdout).trim()
	);

	Pass(cx.input).xok()
}

/// The deploy tree this step reads: the tofu project holding the box's address
/// and key pair, and the single [`LightsailBlock`] declared under the same
/// stack.
#[derive(SystemParam)]
struct ReleaseQuery<'w, 's> {
	stacks: StackQuery<'w, 's>,
	blocks: Query<'w, 's, &'static LightsailBlock>,
}

impl ReleaseQuery<'_, '_> {
	/// The project to read outputs from, and the box to release onto. Several
	/// blocks under one stack is an error rather than a guess: they would have
	/// different management ports and different units.
	fn resolve(
		&self,
		entity: Entity,
	) -> Result<(terra::Project, LightsailBlock)> {
		let project = self.stacks.build_project(entity)?;
		let mut blocks = self
			.stacks
			.declared(entity)?
			.into_iter()
			.filter_map(|child| self.blocks.get(child).ok());
		let block = blocks
			.next()
			.ok_or_else(|| {
				bevyhow!(
					"<LightsailRelease/> found no LightsailBlock under its stack, \
					so there is no box to release onto"
				)
			})?
			.clone();
		if blocks.next().is_some() {
			bevybail!(
				"<LightsailRelease/> found several LightsailBlocks under its \
				stack, so it cannot tell which box to release onto"
			);
		}
		(project, block).xok()
	}
}
