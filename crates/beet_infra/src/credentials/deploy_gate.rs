//! `<stack>/deploy` and `<stack>/deploy --elevated`: the deploy and destroy
//! routes' shell, gated by the repo's deploy credentials.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

/// Request params every gated deploy and destroy route takes beside its own,
/// surfaced in `--help`.
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
/// - **`deploy`** never asks a person for anything. Before its first step it
///   asks every declared credential whether the sealed one holds what the
///   declarations need, and plans the stack against its state (no refresh, a
///   few seconds) when one guards a type the stack declares, for a change no
///   stored credential may make. Either answer refuses the run before anything
///   is written, naming every reason and the command with `--elevated`.
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
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct DeployGate;

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

	/// The overload a gated route dispatches through, in place of
	/// [`ExchangeGroup`]'s own.
	pub fn overload() -> ExchangeOverload {
		ActionOverload::new(Action::new_async(Self::dispatch))
	}

	async fn dispatch(cx: ActionContext<Request>) -> Result<Response> {
		let params = cx.input.parse_params::<ElevationParams>()?;
		let assessed = Self::assess(&cx.caller, &cx.input).await?;
		for note in assessed.iter().flat_map(|one| one.status.notes.iter()) {
			info!("{note}");
		}
		if params.dry_run {
			return Response::ok_text(
				Self::dry_run(&cx.caller, &cx.input, &assessed).await?,
			)
			.xok();
		}
		let blocking = assessed.iter().any(Assessed::blocks);
		match (params.elevated, blocking) {
			(false, false) => ExchangeGroup::run(cx).await,
			(false, true) => Err(Self::refusal(&cx.input, &assessed)),
			(true, _) => Self::run_elevated(cx, assessed, params.roll).await,
		}
	}

	/// Ask every declared credential where it stands, and plan the stack for
	/// the ones that guard a type it declares and are held sealed.
	async fn assess(
		caller: &AsyncEntity,
		input: &Request,
	) -> Result<Vec<Assessed>> {
		let credentials = caller
			.with_state::<DeployCredentialQuery, _>(|_, query| query.all())
			.await??;
		let mut assessed = Vec::new();
		for credential in credentials {
			let status = credential.status(caller).await?;
			assessed.push(Assessed {
				credential,
				status,
				protected: Vec::new(),
			});
		}
		if assessed.is_empty() {
			return assessed.xok();
		}
		let project = terra::Project::resolve(caller).await?;
		let declared = project.config().declared_types();
		let mut guarding = Vec::new();
		for (index, one) in assessed.iter().enumerate() {
			if one.status.needed
				&& declared.iter().any(|kind| one.credential.guards(kind))
				&& one.credential.holds_sealed(caller).await?
			{
				guarding.push(index);
			}
		}
		if guarding.is_empty() {
			return assessed.xok();
		}
		let vars = project.ambient_vars(input.parts())?;
		for change in project.planned_changes(&vars, &[]).await? {
			for index in guarding.iter() {
				if let Some(reason) = assessed[*index]
					.credential
					.protects(&change, project.stack())
				{
					assessed[*index].protected.push(reason);
				}
			}
		}
		assessed.xok()
	}

	/// The refusal of a plain deploy that cannot proceed: every reason, the
	/// human factors an elevated deploy will ask for, and the command.
	fn refusal(input: &Request, assessed: &[Assessed]) -> BevyError {
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
			Self::relay(input)
		)
	}

	/// The dry run: where each credential stands, what it would become, and
	/// whether this deploy needs `--elevated`.
	async fn dry_run(
		caller: &AsyncEntity,
		input: &Request,
		assessed: &[Assessed],
	) -> Result<String> {
		let mut out = String::new();
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
				Self::relay(input)
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
			return ExchangeGroup::run(cx).await;
		}
		let relay = Self::relay(&cx.input);
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
		let ran = Self::run_child(&cx.input, &stripped, &env).await;
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
		stripped: &[&'static str],
		env: &[(SmolStr, SmolStr)],
	) -> Result {
		let args = Self::request_args(input);
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

	/// This request as argv, less this gate's own params and the launch's
	/// knobs (`--entry`, `--stage`, ..), which [`ChildProcess::this_launch`]
	/// and the relay carry from the launch itself: the route as one `a/b`
	/// path, the way a person types it, then each param.
	fn request_args(input: &Request) -> Vec<String> {
		let mut args = input.parts().to_cli_args();
		for param in Self::OWN_PARAMS {
			args.params.remove(param);
		}
		// read only to take the knobs out: they parsed once already, at launch
		BootstrapConfig::take_params(&mut args.params).ok();
		let route = args.path.join("/");
		args.path.clear();
		Some(route)
			.filter(|route| !route.is_empty())
			.into_iter()
			.chain(args.into_args())
			.collect()
	}

	/// The command a person runs in a terminal to deploy this elevated: this
	/// launch's own command line, its gate params replaced by `--elevated`.
	fn relay(input: &Request) -> String {
		env_ext::program()
			.into_iter()
			.chain(
				BootstrapConfig::get()
					.to_argv()
					.unwrap_or_default()
					.into_iter(),
			)
			.chain(Self::request_args(input).into_iter().map(SmolStr::from))
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A refusal names every reason of every credential that blocks, the human
	/// factor each will ask for, and the command to run elevated; a credential
	/// that does not block is not mentioned.
	#[beet_core::test]
	fn refuses_naming_every_reason() {
		let assessed = |stale: &[&str], protected: &[&str]| super::Assessed {
			credential: DeployCredential::new(CloudflareDeployToken::default()),
			status: CredentialStatus {
				needed: true,
				stale: stale.iter().map(ToString::to_string).collect(),
				notes: Vec::new(),
			},
			protected: protected.iter().map(ToString::to_string).collect(),
		};
		super::DeployGate::refusal(&Request::from_cli_str("mail/deploy"), &[
			assessed(&["cloudflare: needs `Zone > DNS Write on a.b`"], &[]),
			assessed(&[], &["this deploy would update `x.lock`"]),
			assessed(&[], &[]),
		])
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
		let args = super::DeployGate::request_args(&request);
		args.iter().any(|arg| arg == "mail/deploy").xpect_true();
		args.iter()
			.any(|arg| arg.starts_with("--dry") || arg == "--roll")
			.xpect_false();
		super::DeployGate::relay(&request)
			.xpect_contains("mail/deploy")
			.xpect_contains("'--note=a b'")
			.xpect_ends_with("--elevated");
	}
}
