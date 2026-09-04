//! The two steps that make a Lightsail box run what the stores now hold: a
//! release after a deploy's apply, and a restart after a content sync.
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
	let (project, block) = resolve_box(&cx, "LightsailRelease").await?;

	let deploy_id = project.deployment().deploy_id().to_string();
	let script = block.release_script(
		project.stack(),
		&deploy_id,
		*release.timeout(),
		*release.poll(),
	);
	info!("releasing {deploy_id}");
	run_gate_script(
		&project,
		&block,
		"release",
		script,
		*release.timeout(),
		*release.poll(),
	)
	.await?;

	Pass(cx.input).xok()
}

/// `<LightsailRestart/>` — after a content sync, bounce the box so it re-reads
/// the repo store.
///
/// A page's bytes are read from the store per request, so a sync is live the
/// moment it lands and this step is not what publishes it. What it buys is the
/// work the box does ONCE at boot: `<RoutesDir/>` discovery, so a page added,
/// deleted or renamed by its `slug` exists as a route, and the entry document
/// itself, so a change to `main.bsx` (its `<Redirect/>` block, its middleware)
/// takes. Without it those need a full deploy to appear.
///
/// Declared under the sync sequence AFTER the `<DirSync/>` that publishes, and
/// BEFORE the `<CloudflarePurgeCache/>`: purging while the old process still
/// serves just re-caches the old responses for the whole edge TTL.
#[derive(Debug, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(LightsailRestartAction)]
pub struct LightsailRestart {
	/// How long to wait for the unit to come back serving, and for the ssh that
	/// drives it. Shorter than [`LightsailRelease`]'s by default: a sync bounces
	/// a box that is already up, rather than meeting one mid cloud-init.
	timeout: Duration,
	/// The gap between attempts, at every gate `timeout` bounds.
	poll: Duration,
}

impl Default for LightsailRestart {
	fn default() -> Self {
		Self {
			timeout: Duration::from_secs(120),
			poll: Duration::from_secs(5),
		}
	}
}

/// Restarts the deployed unit and verifies it came back serving.
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn LightsailRestartAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let restart = cx
		.caller
		.get_cloned::<LightsailRestart>()
		.await
		.unwrap_or_default();
	let (project, block) = resolve_box(&cx, "<LightsailRestart/>").await?;

	let script = block.restart_script(
		project.stack(),
		*restart.timeout(),
		*restart.poll(),
	);
	run_gate_script(
		&project,
		&block,
		"restart",
		script,
		*restart.timeout(),
		*restart.poll(),
	)
	.await?;

	Pass(cx.input).xok()
}

/// The box a step drives: the tofu project holding its address and key pair,
/// and the one [`LightsailBlock`] declared under the same stack.
async fn resolve_box(
	cx: &ActionContext<Request>,
	tag: &'static str,
) -> Result<(terra::Project, LightsailBlock)> {
	cx.caller
		.with_world(move |world, entity| -> Result<_> {
			let project = RenderScope::render(world, entity)?.project()?;
			let block = world.with_state::<ReleaseQuery, _>(|query| {
				query.resolve(entity, tag)
			})?;
			(project, block).xok()
		})
		.await?
}

/// scp `script` to the box's management sshd and run it as root, narrating its
/// stderr through the log and reporting the release it converged on.
async fn run_gate_script(
	project: &terra::Project,
	block: &LightsailBlock,
	name: &str,
	script: String,
	timeout: Duration,
	poll: Duration,
) -> Result<()> {
	let connection =
		SshConnection::from_project(project, block.management_ssh_port())
			.await?;
	connection.wait_for_ready(timeout, poll).await?;

	// the script is a file rather than an argv token: it carries quotes, globs
	// and newlines that would have to survive two shells to arrive intact.
	let local_path = project.work_dir().join(format!("{name}.sh"));
	fs_ext::write_async(&local_path, script.as_bytes()).await?;
	// the ssh user's home dir, not /tmp: a predictable world-writable path is
	// a symlink-planting target, and root runs this script
	let remote_path = format!("beet-{name}.sh");
	connection
		.scp_to(local_path.as_path(), &remote_path)
		.await?;

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
	Ok(())
}

/// The deploy tree these steps read: the tofu project holding the box's address
/// and key pair, and the single [`LightsailBlock`] declared under the same
/// stack.
#[derive(SystemParam)]
struct ReleaseQuery<'w, 's> {
	stacks: StackQuery<'w, 's>,
	blocks: Query<'w, 's, &'static LightsailBlock>,
}

impl ReleaseQuery<'_, '_> {
	/// The box to drive (the project to read outputs from comes from a
	/// [`RenderScope`] render). Several blocks under one stack is an error
	/// rather than a guess: they would have different management ports and
	/// different units. `tag` names the calling step, since both resolve here.
	fn resolve(&self, entity: Entity, tag: &str) -> Result<LightsailBlock> {
		let mut blocks = self
			.stacks
			.declared(entity)?
			.into_iter()
			.filter_map(|child| self.blocks.get(child).ok());
		let block = blocks
			.next()
			.ok_or_else(|| {
				bevyhow!(
					"{tag} found no LightsailBlock under its stack, \
					so there is no box to drive"
				)
			})?
			.clone();
		if blocks.next().is_some() {
			bevybail!(
				"{tag} found several LightsailBlocks under its stack, \
				so it cannot tell which box to drive"
			);
		}
		block.xok()
	}
}
