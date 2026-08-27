//! The declarative apply of everything inside Stalwart's data store.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;

/// `<StalwartProvision/>` — after the apply, make the box that terraform built
/// serve the mail the blocks declared.
///
/// A fresh `0.16` box boots in bootstrap mode: management HTTP on `8080` and
/// nothing else, mail ports silent, authenticating against the recovery admin
/// credential its `stalwart.env` carries. It stays that way until the
/// configuration objects exist, so this step is not a nicety after the apply,
/// it is the half of the install terraform cannot express.
///
/// `8080` is deliberately NOT in the security group. The management endpoint
/// serves plaintext HTTP and answers to a password that provisions the whole
/// server, so it is reached through an ssh port forward over the box's key pair
/// and is unreachable from the internet at any point.
///
/// Idempotent and additive: every object is matched against what the server
/// already has and patched rather than recreated, and nothing is ever deleted.
/// An account the plan no longer declares is left alone, because the failure
/// mode of the alternative is deleting somebody's mailbox on a typo.
#[derive(Debug, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(StalwartProvisionAction)]
pub struct StalwartProvision {
	/// The private half of the key pair the box imported. The box declares a
	/// PUBLIC key and generates nothing, so the private half is the deployer's
	/// own and is never an output of the stack; a leading `~` expands.
	ssh_key: SmolStr,
	/// The local port the management endpoint is forwarded to. Not 8080, so a
	/// developer running a local Stalwart is not quietly provisioned instead.
	local_port: u16,
	/// How long to wait at each gate: for ssh, for the management endpoint, and
	/// for the restarted server to answer on 443. Generous, since a freshly
	/// replaced box is still installing the log agent when the deploy arrives.
	timeout: Duration,
	/// The gap between attempts, at every gate `timeout` bounds.
	poll: Duration,
}

impl Default for StalwartProvision {
	fn default() -> Self {
		Self {
			ssh_key: Self::SSH_KEY.into(),
			local_port: Self::LOCAL_PORT,
			timeout: Duration::from_secs(600),
			poll: Duration::from_secs(5),
		}
	}
}

impl StalwartProvision {
	/// The management port Stalwart serves in bootstrap mode, on loopback at
	/// the far end of the tunnel.
	pub const MANAGEMENT_PORT: u16 = 8080;

	/// The near end of the tunnel.
	pub const LOCAL_PORT: u16 = 18080;

	/// The ssh user of the box's AMI.
	pub const SSH_USER: &'static str = "ec2-user";

	/// Where a deployer's key usually is, which is only a default: a box whose
	/// declared public key came from elsewhere names the private half here.
	pub const SSH_KEY: &'static str = "~/.ssh/id_ed25519";

	/// The unit the box runs, restarted once the objects exist: the mail
	/// listeners only bind at start, so a server configured while running is
	/// still a server serving nothing.
	pub const UNIT: &'static str = "stalwart";
}

/// Applies the plan, restarts the unit and waits for the server to answer on
/// its public port.
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn StalwartProvisionAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let provision = cx
		.caller
		.get_cloned::<StalwartProvision>()
		.await
		.unwrap_or_default();
	let mail = cx
		.caller
		.with_state::<MailQuery, _>(|entity, query| query.resolve(entity))
		.await??;

	// the credentials this step authenticates with and writes into the relay
	// route, all read from parameter store rather than passed in: the box reads
	// the same admin password at every start, so the two cannot drift.
	let region = mail.stack.region().clone();
	let admin_password =
		read_secret(&region, &mail.mail_box.admin_secret_name(&mail.stack))
			.await?;
	let ses = SesCredential {
		username: read_secret(
			&region,
			&mail.mail_box.ses_smtp_user_secret_name(&mail.stack),
		)
		.await?,
		password: read_secret(
			&region,
			&mail.mail_box.ses_smtp_password_secret_name(&mail.stack),
		)
		.await?,
	};

	// the ACME contact and the reports address are the same human, ie whoever
	// answers `postmaster@` on the first domain served.
	let admin_contact = mail
		.domains
		.first()
		.map(|domain| format!("postmaster@{}", domain.domain()))
		.ok_or_else(|| {
			bevyhow!("no mail domain to register an acme account with")
		})?;
	let plan = StalwartPlan::new(
		&mail.mail_box,
		&mail.domains,
		&mail.stack,
		&admin_contact,
		&ses,
	)?;

	// the management endpoint is closed to the internet, so everything below
	// happens through a port forward that dies with this step.
	let connection = SshConnection {
		host: mail.public_ip().await?,
		user: StalwartProvision::SSH_USER.to_string(),
		port: 22,
		key_path: ssh_key_path(provision.ssh_key())?,
	};
	connection
		.wait_for_ready(*provision.timeout(), *provision.poll())
		.await?;
	let _tunnel = connection
		.tunnel(provision.local_port(), StalwartProvision::MANAGEMENT_PORT)
		.await?;
	let origin = format!("http://127.0.0.1:{}", provision.local_port());
	let client = wait_for_management(
		&origin,
		&admin_password,
		*provision.timeout(),
		*provision.poll(),
	)
	.await?;

	apply_plan(&client, &plan, &region, &mail.stack).await?;

	info!(
		"restarting {} so the mail listeners bind",
		StalwartProvision::UNIT
	);
	connection
		.run_command(&format!(
			"sudo -n systemctl restart {}",
			StalwartProvision::UNIT
		))
		.await?;
	drop(_tunnel);

	wait_for_health(&mail.mail_box, *provision.timeout(), *provision.poll())
		.await?;
	Pass(cx.input).xok()
}

/// Converge the server onto `plan`, in dependency order.
///
/// The order is the plan's own shape: the ACME provider before the domains that
/// reference it, the domains before the accounts that live on them, the
/// accounts before the catch-all that names one, and the system settings last
/// because they name a domain.
async fn apply_plan(
	client: &JmapClient,
	plan: &StalwartPlan,
	region: &str,
	stack: &ResolvedStack,
) -> Result {
	client
		.update_singleton("x:SpamSettings", &plan.spam_settings())
		.await?;
	client
		.update_singleton("x:SpamClassifier", &plan.spam_classifier())
		.await?;
	info!("spam filter on, learning from replies and traps");

	let acme = converge(
		client,
		"x:AcmeProvider",
		&["directory"],
		&plan.acme_provider(),
	)
	.await?;
	info!("acme provider ready ({})", StalwartPlan::ACME_DIRECTORY);

	for listener in &plan.listeners {
		converge(client, "x:NetworkListener", &["name"], listener).await?;
	}
	info!("{} listeners declared", plan.listeners.len());

	converge(client, "x:MtaRoute", &["name"], &plan.relay).await?;
	client
		.update_singleton("x:MtaOutboundStrategy", &plan.outbound_strategy())
		.await?;
	info!("outbound relays through {}", plan.relay["address"]);

	let mut default_domain = None;
	for domain in &plan.domains {
		let domain_id =
			converge(client, "x:Domain", &["name"], &domain.object(&acme))
				.await?;
		default_domain.get_or_insert_with(|| domain_id.clone());
		for account in &domain.accounts {
			converge_account(client, account, &domain_id, region, stack)
				.await?;
		}
		// after the accounts, since it names one of them
		if let Some(patch) = domain.catch_all_patch() {
			client.update("x:Domain", &domain_id, &patch).await?;
		}
		info!(
			"{} serving {} mailbox(es)",
			domain.name,
			domain.accounts.len()
		);
	}

	let default_domain = default_domain
		.ok_or_else(|| bevyhow!("the plan declared no domain to default to"))?;
	client
		.update_singleton(
			"x:SystemSettings",
			&plan.system_settings(&default_domain),
		)
		.await?;
	info!("hostname is {}", plan.hostname);
	Ok(())
}

/// Create `object` if no existing one matches on `match_on`, else patch the one
/// that does. Returns the object's id either way.
async fn converge(
	client: &JmapClient,
	object_type: &str,
	match_on: &[&str],
	object: &Value,
) -> Result<String> {
	let existing = client.list(object_type).await?;
	match plan_converge(&existing, match_on, object)? {
		Converge::Create => client.create(object_type, object).await,
		Converge::Unchanged(id) => Ok(id),
		Converge::Patch(id, patch) => {
			client.update(object_type, &id, &patch).await?;
			Ok(id)
		}
	}
}

/// What converging one declared object against what the server holds requires.
#[derive(Debug, Clone, PartialEq)]
enum Converge {
	/// Nothing matches, so this declaration is new.
	Create,
	/// Something matches and already agrees with the declaration.
	Unchanged(String),
	/// Something matches; these properties differ.
	Patch(String, Value),
}

/// Decide, without talking to anything.
///
/// Matching is LOCAL, over every object of the type, rather than a server-side
/// filter: the properties that identify a declaration differ per object type,
/// and a filter the server does not implement returns an empty result, which
/// reads as "nothing exists yet" and silently creates a duplicate of something
/// already there.
///
/// The patch carries only what differs and skips `@`-prefixed keys. Most of an
/// object's properties are server-set (an account's `emailAddress`, a
/// certificate's `notValidAfter`) or defaulted, and sending the whole
/// declaration every time would either clobber them or churn a diff on every
/// deploy.
fn plan_converge(
	existing: &[Value],
	match_on: &[&str],
	object: &Value,
) -> Result<Converge> {
	let Some(found) = existing.iter().find(|candidate| {
		match_on
			.iter()
			.all(|property| candidate[property] == object[property])
	}) else {
		return Ok(Converge::Create);
	};
	let id = found["id"]
		.as_str()
		.ok_or_else(|| bevyhow!("a matched object carried no id"))?
		.to_string();
	let patch = object
		.as_object()
		.map(|declared| {
			declared
				.iter()
				.filter(|(key, value)| {
					!key.starts_with('@') && &&found[key] != value
				})
				.map(|(key, value)| (key.clone(), value.clone()))
				.collect::<serde_json::Map<_, _>>()
		})
		.unwrap_or_default();
	match patch.is_empty() {
		true => Ok(Converge::Unchanged(id)),
		false => Ok(Converge::Patch(id, Value::Object(patch))),
	}
}

/// Create the account if it is not there, minting and parking its password on
/// the way; else patch it, leaving the credential it already has alone.
///
/// A rotation would lock out every client configured against the mailbox, so
/// the password is generated exactly once and everything that needs it (a
/// probe, a human setting up a mail client) reads it back from parameter store.
///
/// The parameter and the account are two independent pieces of state, so which
/// one exists decides a different question: the PARAMETER decides whether to
/// generate, and the ACCOUNT decides whether to write the credential. Reading
/// one to answer the other is how a box rebuilt on a fresh data store gets a
/// mailbox nobody can sign in to.
async fn converge_account(
	client: &JmapClient,
	account: &AccountPlan,
	domain_id: &str,
	region: &str,
	stack: &ResolvedStack,
) -> Result<String> {
	let name = account.secret.name(stack);
	let password = match ssm_ext::get(region, &name).await? {
		Some(password) => password,
		None => {
			let generated =
				EnsureSecret::new(account.secret.clone()).generate()?;
			ssm_ext::create(region, &name, &generated).await?;
			info!("minted the {} mailbox credential", account.name);
			generated.to_string()
		}
	};
	// matched WITHOUT the credential, so an account that already exists never
	// shows a password diff and never has one patched onto it
	let existing = client.list("x:Account").await?;
	let declared = account.object(domain_id, None);
	match plan_converge(&existing, &["name", "domainId"], &declared)? {
		Converge::Create => {
			client
				.create(
					"x:Account",
					&account.object(domain_id, Some(&password)),
				)
				.await
		}
		Converge::Unchanged(id) => Ok(id),
		Converge::Patch(id, patch) => {
			client.update("x:Account", &id, &patch).await?;
			Ok(id)
		}
	}
}

/// The one place a secret is read, so a missing parameter names the step that
/// should have created it rather than failing as an authentication error later.
async fn read_secret(region: &str, name: &str) -> Result<String> {
	ssm_ext::get(region, name).await?.ok_or_else(|| {
		bevyhow!(
			"secret {name} does not exist: the deploy runs <EnsureSecret/> and \
			the apply before this step for exactly this reason"
		)
	})
}

/// The declared private key, with a leading `~` expanded.
///
/// The mail box imports a public key rather than generating a pair, so the
/// private half is the deployer's own: there is no `ssh_private_key` output to
/// read it from the way a Lightsail release does.
fn ssh_key_path(key: &str) -> Result<AbsPathBuf> {
	let expanded = match key.strip_prefix("~/") {
		Some(rest) => {
			let home = env_ext::var("HOME").map_err(|_| {
				bevyhow!(
					"ssh key '{key}' starts with ~ but HOME is unset: name the \
					path in full"
				)
			})?;
			format!("{home}/{rest}")
		}
		None => key.to_string(),
	};
	AbsPathBuf::new(expanded).map_err(Into::into)
}

/// Poll the forwarded management endpoint until it authenticates, which is also
/// the check that the box finished booting and opened its data store.
async fn wait_for_management(
	origin: &str,
	admin_password: &str,
	timeout: Duration,
	poll: Duration,
) -> Result<JmapClient> {
	let attempts = (timeout.as_secs() / poll.as_secs().max(1)).max(1);
	let mut last = None;
	for attempt in 1..=attempts {
		match JmapClient::connect(
			origin,
			StalwartBlock::ADMIN_USER,
			admin_password,
		)
		.await
		{
			Ok(client) => {
				info!("management endpoint ready after {attempt} attempt(s)");
				return Ok(client);
			}
			Err(err) => {
				last = Some(err);
				time_ext::sleep(poll).await;
			}
		}
	}
	bevybail!(
		"the management endpoint never answered: {}",
		last.map(|err| err.to_string()).unwrap_or_default()
	)
}

/// Poll the box's PUBLIC endpoint until it serves the configured server, which
/// is the assertion the whole step exists to make: the listeners bound, the
/// certificate issued and the name resolves.
async fn wait_for_health(
	mail_box: &StalwartBlock,
	timeout: Duration,
	poll: Duration,
) -> Result {
	let url = format!(
		"https://{}{}",
		mail_box.hostname(),
		JmapClient::SESSION_PATH
	);
	let attempts = (timeout.as_secs() / poll.as_secs().max(1)).max(1);
	for attempt in 1..=attempts {
		// unauthenticated, so a 401 is a pass: the question is whether the
		// server is serving TLS on 443 at all, not whether we may sign in.
		match Request::get(&url).send().await {
			Ok(response) if response.status().as_u16() < 500 => {
				info!(
					"{} is serving on 443 after {attempt} attempt(s)",
					mail_box.hostname()
				);
				return Ok(());
			}
			_ => time_ext::sleep(poll).await,
		}
	}
	bevybail!(
		"{} never answered on 443: the certificate may still be issuing, which \
		is the one gate this step cannot hurry",
		mail_box.hostname()
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn existing() -> Vec<Value> {
		vec![serde_json::json!({
			"id": "a1",
			"name": "smtp",
			"protocol": "smtp",
			"bind": ["[::]:25"],
			"tlsImplicit": false,
			// server-set, and named by no declaration
			"maxConnections": 8192,
		})]
	}

	/// Nothing matching means this declaration is new, which is the only case
	/// that may create: a match test that is too strict duplicates an object
	/// the server already has, and for a listener that is a port collision at
	/// the next restart.
	#[beet_core::test]
	fn an_unmatched_declaration_is_created() {
		plan_converge(
			&existing(),
			&["name"],
			&serde_json::json!({ "name": "imaps" }),
		)
		.unwrap()
		.xpect_eq(Converge::Create);
	}

	/// A second deploy that changed nothing must write nothing, or every run
	/// churns the data store and the log says a provision happened when it did
	/// not.
	#[beet_core::test]
	fn an_unchanged_declaration_writes_nothing() {
		plan_converge(
			&existing(),
			&["name"],
			&serde_json::json!({
				"name": "smtp",
				"protocol": "smtp",
				"bind": ["[::]:25"],
				"tlsImplicit": false,
			}),
		)
		.unwrap()
		.xpect_eq(Converge::Unchanged("a1".to_string()));
	}

	/// The patch carries only what differs, and never a property the
	/// declaration does not mention: most of an object is server-set or
	/// defaulted, and resending the lot would clobber it.
	#[beet_core::test]
	fn a_patch_carries_only_what_differs() {
		let Converge::Patch(id, patch) = plan_converge(
			&existing(),
			&["name"],
			&serde_json::json!({
				"name": "smtp",
				"protocol": "smtp",
				"bind": ["[::]:2525"],
				"tlsImplicit": false,
			}),
		)
		.unwrap() else {
			panic!("expected a patch");
		};
		id.as_str().xpect_eq("a1");
		patch.as_object().unwrap().len().xpect_eq(1);
		patch["bind"][0].as_str().unwrap().xpect_eq("[::]:2525");
	}

	/// `@type` names a variant rather than a property, so it identifies the
	/// object and is never patched onto one: a `/set` update carrying it is
	/// rejected outright.
	#[beet_core::test]
	fn the_variant_tag_is_never_patched() {
		let Converge::Patch(_, patch) = plan_converge(
			&[serde_json::json!({ "id": "u1", "name": "pete", "domainId": "d1", "description": "" })],
			&["name", "domainId"],
			&serde_json::json!({
				"@type": "User",
				"name": "pete",
				"domainId": "d1",
				"description": "mailbox",
			}),
		)
		.unwrap() else {
			panic!("expected a patch");
		};
		patch.get("@type").is_none().xpect_true();
		patch["description"].as_str().unwrap().xpect_eq("mailbox");
	}

	/// Matching on several properties is what keeps two domains' `pete@`
	/// apart: name alone would patch one account onto the other's mailbox.
	#[beet_core::test]
	fn every_match_property_must_agree() {
		let accounts = vec![
			serde_json::json!({ "id": "u1", "name": "pete", "domainId": "d1" }),
			serde_json::json!({ "id": "u2", "name": "pete", "domainId": "d2" }),
		];
		plan_converge(
			&accounts,
			&["name", "domainId"],
			&serde_json::json!({ "name": "pete", "domainId": "d2" }),
		)
		.unwrap()
		.xpect_eq(Converge::Unchanged("u2".to_string()));
		plan_converge(
			&accounts,
			&["name", "domainId"],
			&serde_json::json!({ "name": "pete", "domainId": "d3" }),
		)
		.unwrap()
		.xpect_eq(Converge::Create);
	}
}
