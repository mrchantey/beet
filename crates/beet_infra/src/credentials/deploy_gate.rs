//! `<stack>/deploy` and `<stack>/deploy --elevated`: the deploy and destroy
//! routes' shell, gated by the repo's deploy credentials.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

/// Request params every gated deploy and destroy route takes beside its own,
/// declared by [`DeployGate`] at route reach from [`InfraPlugin`].
#[derive(Reflect)]
pub(crate) struct ElevationParams {
	/// Deploy elevated: ask for the human factor of each provider whose deploy
	/// credential needs it, bring the credentials in line with the
	/// declarations, run the deploy with temporary stronger credentials and
	/// delete them. What a plain deploy that cannot proceed names.
	elevated: bool,
	/// With `--elevated`, replace every declared deploy credential's value
	/// even when it is current: the roll.
	roll: bool,
	/// Report what each deploy credential would become and whether this deploy
	/// needs `--elevated`, running nothing and asking for nothing.
	dry_run: bool,
}

/// The shell of a stack's `deploy` and `destroy` routes, which
/// `<DeployRoutes/>` puts on both: the group runs through
/// [`ExchangeGroup::run`] once the repo's deploy credentials
/// ([`DeployCredential`]) allow it.
///
/// A package's verb across its stacks, a [`RouteMatrix`] verb whose every
/// target is gated, carries one too, which [`InfraPlugin`] puts in front of
/// it once the targets resolve ([`RouteMatrixResolved`]): one assessment of
/// every stack it lists, one refusal naming each stack's reasons, at most one
/// human factor per provider, and then each target's own run past its own
/// gate, in the listed order. A verb mixing gated and ungated targets fails
/// the load, since half a gate is none.
///
/// - **`deploy`** never asks a person for anything. Before its first step it
///   asks every declared credential whether the sealed one holds what the
///   declarations need, and plans the verb it gates against the stack's state
///   (no refresh and no lock, a few seconds) when one guards a type the stack
///   declares, for a change no stored credential may make: the apply's plan
///   for a `deploy`, the destroy's for a `destroy`, so a teardown is judged on
///   the deletions it makes. Either answer refuses the run before anything is
///   written, naming every reason and the command with `--elevated`.
/// - **`deploy --elevated`** asks only for the human factor of each provider
///   that needs one (a stale credential, a protected change, `--roll`),
///   converges those credentials, and runs the deploy as this launch in a
///   child that loads the credentials just sealed, with whatever temporary
///   credential a protected change needs in its environment, deleted once it
///   exits however it exits. With nothing to elevate it deploys as it would
///   plainly.
/// - **`--dry-run`** reports, asking for nothing, what each credential would
///   become and whether this deploy needs `--elevated`.
///
/// No provider is named here: each declared credential answers for itself, so
/// a provider added to an entry adds a credential to the same two commands.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub enum DeployGate {
	/// Gates the verb that converges the stack, assessing the apply's plan.
	#[default]
	Deploy,
	/// Gates the verb that tears the stack down, assessing the destroy's
	/// plan: a `delete` for every resource the state holds, so a protected
	/// removal is what it looks for rather than a change the declarations
	/// made.
	Destroy,
}

/// What a gated route runs once its [`DeployGate`] lets a request through:
/// the route's own work, which the gate's overload stands in front of.
#[derive(Clone, Component)]
pub struct GatePass(Action<Request, Response>);

impl GatePass {
	/// Run `cx.caller`'s pass.
	async fn run(cx: ActionContext<Request>) -> Result<Response> {
		let pass = cx.caller.get(|pass: &GatePass| pass.0.clone()).await?;
		cx.caller.call_detached(pass, cx.input).await
	}
}

/// What one credential answered for this run.
struct Assessed {
	credential: DeployCredential,
	status: CredentialStatus,
	/// Why each planned change it protects needs elevation.
	protected: Vec<String>,
}

impl Assessed {
	/// Whether this credential has to be elevated for the run to proceed.
	fn blocks(&self) -> bool {
		!self.status.stale.is_empty() || !self.protected.is_empty()
	}

	/// Every reason it blocks, then every note.
	fn reasons(&self) -> impl Iterator<Item = &String> {
		self.status.stale.iter().chain(self.protected.iter())
	}
}

impl DeployGate {
	/// The params a refusal and an elevated run strip from the request they
	/// relay or re-run, under either spelling a request may carry them.
	const OWN_PARAMS: [&'static str; 4] =
		["elevated", "roll", "dry-run", "dry_run"];

	/// This gate in front of `pass`: the gate, the overload the route
	/// dispatches through in place of its own, and what it runs once it lets
	/// a request through.
	pub fn guard(self, pass: Action<Request, Response>) -> impl Bundle {
		(
			self,
			ExchangeOverload::new(Action::new_async(Self::dispatch)),
			GatePass(pass),
		)
	}

	async fn dispatch(cx: ActionContext<Request>) -> Result<Response> {
		let params = cx.input.parse_params::<ElevationParams>()?;
		// what this route takes, so a command line rebuilt from the request
		// repeats no param nothing reads, ie a typo
		let declared = cx
			.caller
			.with_state::<ParamsQuery, _>(|entity, query| query.of(entity))
			.await??;
		let stacks = Self::stacks(&cx.caller).await?;
		let assessed = cx
			.caller
			.get_cloned::<Self>()
			.await?
			.assess(&cx.caller, &cx.input, &stacks)
			.await?;
		for note in assessed.iter().flat_map(|one| one.status.notes.iter()) {
			info!("{note}");
		}
		if params.dry_run {
			return Response::ok_text(
				Self::dry_run(
					&cx.caller, &cx.input, &declared, &stacks, &assessed,
				)
				.await?,
			)
			.xok();
		}
		let blocking = assessed.iter().any(Assessed::blocks);
		match (params.elevated, blocking) {
			(false, false) => GatePass::run(cx).await,
			(false, true) => {
				Err(Self::refusal(&cx.input, &declared, &assessed))
			}
			(true, _) => {
				Self::run_elevated(cx, &declared, assessed, params.roll).await
			}
		}
	}

	/// A gated matrix verb's pass: each target's own pass, in the listed
	/// order, since this gate already assessed every stack they deploy.
	async fn run_matrix(cx: ActionContext<Request>) -> Result<Response> {
		RouteMatrixStep::run_through(cx, Action::new_async(GatePass::run)).await
	}

	/// The stacks this route deploys, each by the route entity it resolves
	/// from: a stack's own route unlabelled, or each target of a matrix verb
	/// labelled by its namespace, in the listed order.
	async fn stacks(
		caller: &AsyncEntity,
	) -> Result<Vec<(Option<SmolStr>, AsyncEntity)>> {
		let targets = caller
			.with_state::<Query<&RouteMatrixStep>, _>(|entity, steps| {
				steps.get(entity).ok().map(|step| step.targets.clone())
			})
			.await?;
		match targets {
			Some(targets) => targets
				.into_iter()
				.map(|target| {
					(
						target.path.first().cloned(),
						caller.world().entity(target.entity),
					)
				})
				.collect(),
			None => vec![(None, caller.clone())],
		}
		.xok()
	}

	/// Ask every declared credential where it stands, once, and plan the
	/// gated verb on each of `stacks` that declares a type a credential held
	/// sealed guards.
	async fn assess(
		self,
		caller: &AsyncEntity,
		input: &Request,
		stacks: &[(Option<SmolStr>, AsyncEntity)],
	) -> Result<Vec<Assessed>> {
		let credentials = caller
			.with_state::<DeployCredentialQuery, _>(|_, query| query.all())
			.await??;
		let mut assessed = Vec::new();
		// only a credential held sealed is held to what a stored one may do:
		// one passed for the command is the provider's to judge
		let mut held = Vec::new();
		for credential in credentials {
			let mut status = credential.status(caller).await?;
			match credential.holding(caller).await? {
				Holding::Sealed if status.needed => held.push(assessed.len()),
				// running as another credential, the sealed one's
				// shortcomings are worth saying and block nothing
				Holding::Another => {
					status.notes.extend(status.stale.drain(..).map(|reason| {
						format!("{reason}, though this launch runs as another")
					}))
				}
				_ => {}
			}
			assessed.push(Assessed {
				credential,
				status,
				protected: Vec::new(),
			});
		}
		if held.is_empty() {
			return assessed.xok();
		}
		for (label, stack) in stacks {
			let project = terra::Project::resolve(stack).await?;
			let declared = project.config().declared_types();
			let guarding = held
				.iter()
				.copied()
				.filter(|index| {
					declared
						.iter()
						.any(|kind| assessed[*index].credential.guards(kind))
				})
				.collect::<Vec<_>>();
			if guarding.is_empty() {
				continue;
			}
			for change in self.planned_changes(&project, input).await? {
				for index in guarding.iter() {
					if let Some(reason) = assessed[*index]
						.credential
						.protects(&change, project.stack())
					{
						assessed[*index].protected.push(match label {
							Some(label) => format!("{label}: {reason}"),
							None => reason,
						});
					}
				}
			}
		}
		assessed.xok()
	}

	/// The writes the gated verb would make, planned against the stack's
	/// state alone with the vars that verb runs with.
	async fn planned_changes(
		self,
		project: &terra::Project,
		input: &Request,
	) -> Result<Vec<tofu::PlannedChange>> {
		match self {
			Self::Deploy => {
				project
					.planned_changes(&project.ambient_vars(input.parts())?, &[])
					.await
			}
			Self::Destroy => project.planned_destroy().await,
		}
	}

	/// The refusal of a plain deploy that cannot proceed: every reason, the
	/// human factors an elevated deploy will ask for, and the command.
	fn refusal(
		input: &Request,
		declared: &ParamsPattern,
		assessed: &[Assessed],
	) -> BevyError {
		let blocking = assessed
			.iter()
			.filter(|one| one.blocks())
			.collect::<Vec<_>>();
		bevyhow!(
			"this deploy needs `--elevated`, and nothing is written yet:\n{}\n\n\
			Run it elevated in a terminal, which asks for {}, brings the \
			credentials in line with the declarations, runs this deploy and \
			deletes any temporary credential:\n\n\t{}",
			blocking
				.iter()
				.flat_map(|one| one.reasons())
				.map(|reason| format!("- {reason}"))
				.collect::<Vec<_>>()
				.join("\n"),
			blocking
				.iter()
				.map(|one| one.credential.human_factor())
				.collect::<Vec<_>>()
				.join(" and "),
			Self::relay(input, declared)
		)
	}

	/// The dry run: the stacks a matrix verb runs, where each credential
	/// stands, what it would become, and whether this deploy needs
	/// `--elevated`.
	async fn dry_run(
		caller: &AsyncEntity,
		input: &Request,
		declared: &ParamsPattern,
		stacks: &[(Option<SmolStr>, AsyncEntity)],
		assessed: &[Assessed],
	) -> Result<String> {
		let mut out = String::new();
		let labels = stacks
			.iter()
			.filter_map(|(label, _)| label.as_ref())
			.map(|label| format!("`{label}`"))
			.collect::<Vec<_>>();
		if !labels.is_empty() {
			out.push_str(&format!(
				"runs {}, in that order\n\n",
				labels.join(", ")
			));
		}
		for one in assessed {
			out.push_str(&format!("{}: {}\n", one.credential.id(), match (
				one.status.needed,
				one.blocks()
			) {
				(false, _) => "not needed by this launch",
				(true, false) => "current",
				(true, true) => "needs `--elevated`",
			}));
			for line in one.reasons().chain(one.status.notes.iter()) {
				out.push_str(&format!("- {line}\n"));
			}
			if one.status.needed {
				out.push_str(&format!(
					"\n{}\n",
					one.credential.describe(caller).await?
				));
			}
		}
		if assessed.iter().any(Assessed::blocks) {
			out.push_str(&format!(
				"this deploy needs `--elevated`:\n\n\t{}\n",
				Self::relay(input, declared)
			));
		}
		out.xok()
	}

	/// Elevate every credential that needs it (or every needed one on
	/// `roll`), then run this deploy as this launch in a child holding what
	/// each elevation produced, and undo the temporary credentials however
	/// the child exits.
	async fn run_elevated(
		cx: ActionContext<Request>,
		declared: &ParamsPattern,
		assessed: Vec<Assessed>,
		roll: bool,
	) -> Result<Response> {
		let elevating = assessed
			.iter()
			.filter(|one| one.blocks() || (roll && one.status.needed))
			.collect::<Vec<_>>();
		if elevating.is_empty() {
			info!(
				"nothing this deploy declares needs elevation, so it runs as it \
				would plainly"
			);
			return GatePass::run(cx).await;
		}
		let relay = Self::relay(&cx.input, declared);
		let mut report = Vec::new();
		let mut env = Vec::<(SmolStr, SmolStr)>::new();
		let mut cleanups = Vec::new();
		let mut stripped = Vec::<&'static str>::new();
		for one in elevating {
			let elevation = one
				.credential
				.elevate(&cx.caller, ElevationAsk {
					roll,
					run: !one.protected.is_empty(),
					relay: relay.clone(),
				})
				.await?;
			report.extend(
				elevation
					.report
					.into_iter()
					.map(|line| format!("{}: {line}", one.credential.id())),
			);
			env.extend(elevation.env);
			cleanups.extend(elevation.cleanup);
			stripped.extend(one.credential.records());
		}
		let ran = Self::run_child(&cx.input, declared, &stripped, &env).await;
		for cleanup in cleanups {
			match cleanup.await {
				Ok(line) => report.push(line),
				Err(err) => report.push(format!("cleanup failed: {err}")),
			}
		}
		let report = report.join("\n");
		match ran {
			Ok(()) => Response::ok_text(format!("{report}\n")).xok(),
			Err(err) => bevybail!("{err}\n{report}"),
		}
	}

	/// Re-run this request as this launch, less this gate's own params: every
	/// `stripped` record removed from the child's environment, so the child's
	/// launch loads the values the elevation just sealed rather than
	/// inheriting the ones this launch loaded before them, then `env` set.
	async fn run_child(
		input: &Request,
		declared: &ParamsPattern,
		stripped: &[&'static str],
		env: &[(SmolStr, SmolStr)],
	) -> Result {
		let args = Self::request_args(input, declared);
		let process = env.iter().fold(
			stripped
				.iter()
				.fold(ChildProcess::this_launch(&args)?, |process, record| {
					process.without_env(*record)
				}),
			|process, (key, value)| {
				process
					.with_secret(value.clone())
					.with_env(key.clone(), value.clone())
			},
		);
		let status = process.spawn()?.status().await?;
		match status.success() {
			true => OK,
			false => bevybail!(
				"`{}` exited with {}",
				args.join(" "),
				status.code().unwrap_or(-1)
			),
		}
	}

	/// This request as argv, less this gate's own params, every launch knob
	/// (`--server`, `--port`, ..), which [`ChildProcess::this_launch`] and the
	/// relay carry from the launch itself, and every param the route does not
	/// take, so a typo is never repeated back: the route as one `a/b` path, the
	/// way a person types it, then each param.
	fn request_args(input: &Request, declared: &ParamsPattern) -> Vec<String> {
		let mut args = input.parts().to_cli_args();
		args.params = declared.recognised(&args.params);
		for param in Self::OWN_PARAMS {
			args.params.remove(param);
		}
		for knob in BootstrapConfig::arg_names() {
			args.params.remove(knob.as_str());
		}
		let route = args.path.join("/");
		args.path.clear();
		Some(route)
			.filter(|route| !route.is_empty())
			.into_iter()
			.chain(args.into_args())
			.collect()
	}

	/// The command a person runs in a terminal to deploy this elevated: this
	/// launch's knobs and the request's recognised params, its gate params
	/// replaced by `--elevated`. Composed from what was parsed rather than the
	/// raw argv, which may hold the very typo that led here.
	fn relay(input: &Request, declared: &ParamsPattern) -> String {
		env_ext::program()
			.into_iter()
			.chain(
				BootstrapConfig::get()
					.to_argv()
					.unwrap_or_default()
					.into_iter(),
			)
			.chain(
				Self::request_args(input, declared)
					.into_iter()
					.map(SmolStr::from),
			)
			.chain([SmolStr::from("--elevated")])
			.map(|arg| Self::quote(&arg))
			.collect::<Vec<_>>()
			.join(" ")
	}

	/// `arg` as a shell reads it back: bare when plain, else single quoted.
	fn quote(arg: &str) -> String {
		let plain = !arg.is_empty()
			&& arg.chars().all(|char| {
				char.is_ascii_alphanumeric() || "-_=./:,@%+".contains(char)
			});
		match plain {
			true => arg.to_string(),
			false => format!("'{}'", arg.replace('\'', r"'\''")),
		}
	}
}

/// Observer: put a [`DeployGate`] in front of a [`RouteMatrix`] verb whose
/// every target is gated, once its targets resolve, so a package's verb passes
/// one gate for every stack it lists; a verb mixing gated and ungated targets
/// fails the load naming both.
pub(crate) fn gate_matrix(
	ev: On<RouteMatrixResolved>,
	steps: Query<&RouteMatrixStep>,
	gated: Query<&DeployGate>,
	mut commands: Commands,
) {
	let Ok(step) = steps.get(ev.entity) else {
		return;
	};
	let (gates, open): (Vec<_>, Vec<_>) = step
		.targets
		.iter()
		.partition(|target| gated.contains(target.entity));
	if gates.is_empty() {
		return;
	}
	if open.is_empty() {
		// a verb tearing its stacks down plans their destroys
		let gate = gates
			.iter()
			.filter_map(|target| gated.get(target.entity).ok())
			.find(|gate| **gate == DeployGate::Destroy)
			.copied()
			.unwrap_or_default();
		commands
			.entity(ev.entity)
			.insert(gate.guard(Action::new_async(DeployGate::run_matrix)));
		return;
	}
	let names = |targets: &[&RouteMatrixTarget]| {
		targets
			.iter()
			.map(|target| format!("`{}`", target.path.join("/")))
			.collect::<Vec<_>>()
			.join(", ")
	};
	let err = bevyhow!(
		"the matrix verb `{}` lists {}, gated by the deploy credentials, \
		beside {}, which {} not: half a gate is none, so list them under \
		separate verbs",
		step.verb,
		names(&gates),
		names(&open),
		match open.len() {
			1 => "is",
			_ => "are",
		}
	);
	error!("{err}");
	commands
		.entity(ev.root)
		.insert(TemplateError::new(CloneError::new(err)));
}

#[cfg(test)]
mod test {
	use super::ElevationParams;
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_router::prelude::*;

	/// A credential answering with fixed reasons that counts how often it is
	/// asked, guarding nothing so no stack is planned, and held as `holding`.
	#[derive(Clone)]
	struct Counted {
		stale: Vec<String>,
		asked: Store<usize>,
		holding: Holding,
	}

	impl Counted {
		/// Held sealed, so its reasons block.
		fn sealed(stale: &[&str], asked: Store<usize>) -> Self {
			Self {
				stale: stale.iter().map(ToString::to_string).collect(),
				asked,
				holding: Holding::Sealed,
			}
		}
	}

	impl DeployCredentialProvider for Counted {
		fn id(&self) -> &'static str { "counted" }

		fn human_factor(&self) -> &'static str { "a counted code" }

		fn records(&self) -> &'static [&'static str] { &["COUNTED"] }

		fn guards(&self, _declared: &str) -> bool { false }

		fn protects(
			&self,
			_change: &tofu::PlannedChange,
			_stack: &ResolvedStack,
		) -> Option<String> {
			None
		}

		fn status(
			&self,
			_caller: AsyncEntity,
		) -> SendBoxedFuture<Result<CredentialStatus>> {
			self.asked.set(self.asked.get() + 1);
			let stale = self.stale.clone();
			Box::pin(async move {
				CredentialStatus {
					needed: true,
					stale,
					notes: Vec::new(),
				}
				.xok()
			})
		}

		fn elevate(
			&self,
			_caller: AsyncEntity,
			_ask: ElevationAsk,
		) -> SendBoxedFuture<Result<CredentialElevation>> {
			Box::pin(async { bevybail!("a test elevates nothing") })
		}

		fn describe(
			&self,
			_caller: AsyncEntity,
		) -> SendBoxedFuture<Result<String>> {
			Box::pin(async { "counted: as declared\n".to_string().xok() })
		}

		fn holding(
			&self,
			_caller: AsyncEntity,
		) -> SendBoxedFuture<Result<Holding>> {
			let holding = self.holding;
			Box::pin(async move { holding.xok() })
		}
	}

	/// A package as an entry declares one: the stacks `site` and `mail`, each
	/// with a lifecycle and its state kept locally, a plain `other/deploy`,
	/// and `all` over `matrix`, settled as a load settles it, `counted` its
	/// one declared credential.
	fn package(
		counted: Option<Counted>,
		matrix: RouteMatrix,
	) -> (World, Entity, TempDir) {
		let mut world = (AsyncPlugin, RouterPlugin, InfraPlugin).into_world();
		let (deployment, dir) = Deployment::default_local();
		world.insert_resource(deployment);
		world.init_resource::<PackageConfig>();
		if let Some(counted) = counted {
			world.spawn(DeployCredential::new(counted));
		}
		let stack = |name: &str| {
			(
				PathPartial::new(name),
				Stack::new(name),
				AwsRegion::new("us-west-2"),
			)
		};
		let root = world
			.spawn((Router::with_defaults(), children![
				stack("site"),
				stack("mail"),
				(PathPartial::new("other"), children![(
					PathPartial::new("deploy"),
					Action::<Request, Response>::new_async(
						async |_: ActionContext<Request>| Response::ok().xok()
					),
				)]),
				(PathPartial::new("all"), matrix),
			]))
			.flush();
		let stacks = world
			.query_filtered::<Entity, With<Stack>>()
			.iter(&world)
			.collect::<Vec<_>>();
		for stack in stacks {
			let deploy = world.spawn(Group).flush();
			let destroy = world.spawn(Group).flush();
			world
				.entity_mut(stack)
				.insert_template(DeployRoutes {
					deploy: Some(deploy),
					destroy: Some(destroy),
				})
				.unwrap();
		}
		world.flush();
		world
			.entity_mut(root)
			.trigger_target(|entity| Ready { entity });
		world.flush();
		(world, root, dir)
	}

	/// A matrix verb whose every target is gated is gated too, taking the
	/// gate's params; one whose targets are ungated stays as it was.
	#[beet_core::test]
	fn gates_a_verb_whose_targets_are_all_gated() {
		let (world, root, _dir) = package(
			None,
			RouteMatrix::new(["site", "mail"], ["plan", "deploy"]),
		);
		world.entity(root).get::<TemplateError>().xpect_none();
		let tree = RouteTree::of(&world, root).unwrap();
		let deploy = tree.find(&["all", "deploy"]).unwrap();
		world
			.entity(deploy.entity)
			.contains::<DeployGate>()
			.xpect_true();
		deploy.params.contains("elevated").xpect_true();
		world
			.entity(tree.find(&["all", "plan"]).unwrap().entity)
			.contains::<DeployGate>()
			.xpect_false();
	}

	/// A teardown is planned as one: `destroy` carries a
	/// [`DeployGate::Destroy`], and a matrix verb over teardowns inherits it,
	/// where `deploy` plans forward.
	#[beet_core::test]
	fn a_teardown_plans_its_destroy() {
		let (world, root, _dir) = package(
			None,
			RouteMatrix::new(["site", "mail"], ["deploy", "destroy"]),
		);
		let tree = RouteTree::of(&world, root).unwrap();
		let gate = |path: &[&str]| {
			*world
				.entity(tree.find(path).unwrap().entity)
				.get::<DeployGate>()
				.unwrap()
		};
		gate(&["site", "destroy"]).xpect_eq(DeployGate::Destroy);
		gate(&["site", "deploy"]).xpect_eq(DeployGate::Deploy);
		gate(&["all", "destroy"]).xpect_eq(DeployGate::Destroy);
		gate(&["all", "deploy"]).xpect_eq(DeployGate::Deploy);
	}

	/// Half a gate is none: a verb listing a gated target beside an ungated
	/// one fails the load naming both.
	#[beet_core::test]
	fn a_mixed_verb_fails_the_load() {
		let (world, root, _dir) =
			package(None, RouteMatrix::new(["site", "other"], ["deploy"]));
		world
			.entity(root)
			.get::<TemplateError>()
			.unwrap()
			.error
			.to_string()
			.xpect_contains("`site/deploy`")
			.xpect_contains("`other/deploy`");
	}

	/// A package's deploy asks each credential once however many stacks it
	/// lists, then runs each stack past its own gate rather than through it;
	/// a stack's own deploy still asks for itself.
	#[beet_core::test]
	async fn assesses_a_package_once() {
		let asked = Store::new(0);
		let (mut world, root, _dir) = package(
			Some(Counted::sealed(&[], asked)),
			RouteMatrix::new(["site", "mail"], ["deploy"]),
		);
		world
			.entity_mut(root)
			.exchange(Request::get("all/deploy"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
		asked.get().xpect_eq(1);
		world
			.entity_mut(root)
			.exchange(Request::get("site/deploy"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
		asked.get().xpect_eq(2);
	}

	/// A package's deploy that cannot proceed refuses once, naming every
	/// reason and the one package command to run elevated.
	#[beet_core::test]
	async fn refuses_a_package_once() {
		let asked = Store::new(0);
		let (mut world, root, _dir) = package(
			Some(Counted::sealed(&["counted: needs `x`"], asked)),
			RouteMatrix::new(["site", "mail"], ["deploy"]),
		);
		let response = world
			.entity_mut(root)
			.exchange(Request::get("all/deploy"))
			.await;
		response.status().is_ok().xpect_false();
		response
			.text()
			.await
			.unwrap()
			.xpect_contains("- counted: needs `x`")
			.xpect_contains("all/deploy --elevated");
		asked.get().xpect_eq(1);
	}

	/// Running as another credential (an administrator's, CI's), the sealed
	/// one's shortcomings block nothing; running as none at all, they do,
	/// since there is nothing to run as.
	#[beet_core::test]
	async fn only_the_sealed_credential_is_held_to_its_grants() {
		let deploy = async |holding: Holding| {
			let (mut world, root, _dir) = package(
				Some(Counted {
					holding,
					..Counted::sealed(&["counted: needs `x`"], Store::new(0))
				}),
				RouteMatrix::new(["site"], ["deploy"]),
			);
			world
				.entity_mut(root)
				.exchange(Request::get("site/deploy"))
				.await
				.status()
		};
		deploy(Holding::Another).await.xpect_eq(StatusCode::OK);
		deploy(Holding::Sealed).await.is_ok().xpect_false();
		deploy(Holding::None).await.is_ok().xpect_false();
	}

	/// A package's dry run names every stack it lists, in order, and answers
	/// for each credential without running any of them.
	#[beet_core::test]
	async fn dry_runs_every_listed_stack() {
		let asked = Store::new(0);
		let (mut world, root, _dir) = package(
			Some(Counted::sealed(&[], asked)),
			RouteMatrix::new(["site", "mail"], ["deploy"]),
		);
		world
			.entity_mut(root)
			.exchange_str(Request::from_cli_str("all/deploy --dry-run"))
			.await
			.xpect_contains("runs `site`, `mail`, in that order")
			.xpect_contains("counted: current")
			.xpect_contains("counted: as declared");
		asked.get().xpect_eq(1);
	}

	/// A param a gated route might take beside the gate's own.
	#[derive(Reflect)]
	#[allow(dead_code)]
	struct NoteParams {
		note: Option<String>,
	}

	/// What a gated route with a `--note` param takes.
	fn declared() -> ParamsPattern {
		ParamsPattern::from_declarations([(
			Entity::PLACEHOLDER,
			ParamsScope::Route,
			&ParamsDeclaration::new::<
				super::DeployGate,
				(ElevationParams, NoteParams),
			>(),
		)])
		.unwrap()
	}

	/// One credential's answer, blocking for each `stale` or `protected`.
	fn assessed(stale: &[&str], protected: &[&str]) -> super::Assessed {
		super::Assessed {
			credential: DeployCredential::new(
				CloudflareDeployCredential::default(),
			),
			status: CredentialStatus {
				needed: true,
				stale: stale.iter().map(ToString::to_string).collect(),
				notes: Vec::new(),
			},
			protected: protected.iter().map(ToString::to_string).collect(),
		}
	}

	/// A refusal names every reason of every credential that blocks, the human
	/// factor each will ask for, and the command to run elevated; a credential
	/// that does not block is not mentioned.
	#[beet_core::test]
	fn refuses_naming_every_reason() {
		super::DeployGate::refusal(
			&Request::from_cli_str("mail/deploy"),
			&declared(),
			&[
				assessed(&["cloudflare: needs `Zone > DNS Write on a.b`"], &[]),
				assessed(&[], &["this deploy would update `x.lock`"]),
				assessed(&[], &[]),
			],
		)
		.to_string()
		.xpect_contains("nothing is written yet")
		.xpect_contains("- cloudflare: needs `Zone > DNS Write on a.b`")
		.xpect_contains("- this deploy would update `x.lock`")
		.xpect_contains("the Cloudflare dashboard login")
		.xpect_contains("mail/deploy --elevated");
	}

	/// A relayed command is this request less the gate's own params, with
	/// `--elevated` appended, and a value with spaces quoted.
	#[beet_core::test]
	fn relays_the_deploy_elevated() {
		let request =
			Request::from_cli_str("mail/deploy --dry-run --roll --note='a b'");
		let args = super::DeployGate::request_args(&request, &declared());
		args.iter().any(|arg| arg == "mail/deploy").xpect_true();
		args.iter()
			.any(|arg| arg.starts_with("--dry") || arg == "--roll")
			.xpect_false();
		super::DeployGate::relay(&request, &declared())
			.xpect_contains("mail/deploy")
			.xpect_contains("'--note=a b'")
			.xpect_ends_with("--elevated");
	}

	/// REGRESSION: the gate assessed a `destroy` on the apply's plan, which
	/// for a standing stack holds no deletion, so a teardown no stored
	/// credential may make (a prod bucket's) passed the gate to fail halfway
	/// through the destroy. Each gate plans its own verb's change set.
	#[beet_core::test(timeout_ms = 60_000)]
	async fn assesses_the_verb_it_gates() {
		terra::tofu::test_support::require().await.unwrap();
		StateEncryption::ensure_test_passphrase();
		let (stack, deployment, dir) = ResolvedStack::default_local();
		let mut config = deployment.create_config(&stack);
		config
			.add_untyped_resource(
				"terraform_data",
				"box",
				&serde_json::json!({ "input": "box" }),
			)
			.unwrap();
		let project = terra::Project::new(
			stack,
			deployment,
			config,
			LocalBackend::new(dir.join("state")).into(),
		);
		project.apply().await.unwrap();
		let request = Request::from_cli_str("destroy");
		super::DeployGate::Deploy
			.planned_changes(&project, &request)
			.await
			.unwrap()
			.xpect_empty();
		super::DeployGate::Destroy
			.planned_changes(&project, &request)
			.await
			.unwrap()
			.into_iter()
			.map(|change| (change.address, change.action))
			.collect::<Vec<_>>()
			.xpect_eq(vec![(
				SmolStr::from("terraform_data.box"),
				SmolStr::from("delete"),
			)]);
	}

	/// REGRESSION: `deploy --elevate` (the `d` missing) ran a plain deploy whose
	/// refusal told the user to run `deploy --elevate --elevated`, echoing the
	/// typo into the command it suggested. The suggestion is composed from the
	/// params the route declares, so an unrecognised one never reaches it.
	#[beet_core::test]
	fn refusal_repeats_no_unrecognised_param() {
		let request = Request::from_cli_str("deploy --elevate --note=ship");
		super::DeployGate::request_args(&request, &declared())
			.xpect_eq(vec!["deploy".to_string(), "--note=ship".to_string()]);
		super::DeployGate::refusal(&request, &declared(), &[assessed(
			&["cloudflare: stale"],
			&[],
		)])
		.to_string()
		.xpect_contains("deploy --note=ship --elevated")
		.xnot()
		.xpect_contains("--elevate ");
	}
}
