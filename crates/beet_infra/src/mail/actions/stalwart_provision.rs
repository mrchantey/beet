//! The declarative apply of everything inside Stalwart's data store.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;
use serde_json::json;

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

	/// A declared private key path, with a leading `~` expanded.
	///
	/// The mail box imports a public key rather than generating a pair, so the
	/// private half is the deployer's own: there is no `ssh_private_key` output
	/// to read it from the way a Lightsail release does. Every mail step that
	/// reaches the box reads it through here.
	pub fn key_path(key: &str) -> Result<AbsPathBuf> {
		let expanded = match key.strip_prefix("~/") {
			Some(rest) => {
				let home = env_ext::var("HOME").map_err(|_| {
					bevyhow!(
						"ssh key '{key}' starts with ~ but HOME is unset: name \
						the path in full"
					)
				})?;
				format!("{home}/{rest}")
			}
			None => key.to_string(),
		};
		AbsPathBuf::new(expanded).map_err(Into::into)
	}
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

	// the credentials this step writes into the relay route, read from
	// parameter store rather than passed in: the box reads the same values, so
	// the two cannot drift.
	let region = mail.stack.region().clone();
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
		.serving()
		.next()
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

	// ssh is needed either way: the unit is restarted over it, and the tunnel
	// (if one is opened at all) rides the same connection.
	let connection = SshConnection {
		host: mail.public_ip().await?,
		user: StalwartProvision::SSH_USER.to_string(),
		port: 22,
		key_path: StalwartProvision::key_path(provision.ssh_key())?,
	};
	connection
		.wait_for_ready(*provision.timeout(), *provision.poll())
		.await?;

	let mut management =
		Management::open(&connection, &provision, &plan, &mail, &region)
			.await?;
	let domain_ids =
		apply_plan(management.client(), &plan, &region, &mail.stack).await?;

	info!(
		"restarting {} so the mail listeners bind",
		StalwartProvision::UNIT
	);
	management.restart(&connection, &provision).await?;

	// certificates come from AcmeRenewal tasks, and a task executes only while
	// the process that accepted it keeps running: one scheduled moments before
	// the restart above dies pending with the process. So issuance is asked
	// for AFTER the last restart and waited on: 443 serving the real
	// certificate is the deploy's proof, not its hope.
	converge_certificates(
		management.client(),
		&plan,
		&domain_ids,
		*provision.timeout(),
		*provision.poll(),
	)
	.await?;

	// only now, with a certificate on 443, may the plaintext management port go
	// away: it is the channel everything above ran on when the box was fresh.
	// The restart that unbinds it is therefore the LAST use of this channel —
	// reconnecting on it would poll a port this step just deleted — so the
	// management value is dropped first and 443 is the only thing asked after.
	let retired = retire_listeners(management.client(), &plan).await?;
	drop(management);
	if retired {
		info!(
			"restarting {} so the retired listeners unbind",
			StalwartProvision::UNIT
		);
		connection.run_command(&restart_command()).await?;
	}

	wait_for_health(&mail.mail_box, *provision.timeout(), *provision.poll())
		.await?;
	Pass(cx.input).xok()
}

/// The channel this run reached the management API on, and how to get it back
/// after a restart.
///
/// There are two, and which one is available says exactly how far the box has
/// been commissioned. The PUBLIC endpoint is the steady state: 443, a real
/// certificate, and the administrator account the server created for itself.
/// The BOOTSTRAP channel is a forwarded plaintext port authenticated by the
/// recovery credential, which exists only while the data store is unclaimed and
/// which the box stops writing into `stalwart.env` the moment it is claimed.
///
/// Holding the tunnel here is what keeps its lifetime honest: it is dropped
/// with this value, after the last call that needs it.
struct Management {
	client: JmapClient,
	origin: String,
	user: String,
	password: String,
	/// The ssh port forward, held open for as long as `origin` names its near
	/// end, and closed when this value is dropped. `None` for the public
	/// endpoint. Never read: HOLDING it is the whole job, which is what keeps
	/// the forward's lifetime tied to the channel that uses it rather than to
	/// whoever remembered to close it.
	#[expect(dead_code, reason = "held open for its Drop")]
	tunnel: Option<ChildHandle>,
}

impl Management {
	/// Reach the server the furthest-along way it can be reached, claiming the
	/// data store first if it has never been claimed.
	async fn open(
		connection: &SshConnection,
		provision: &StalwartProvision,
		plan: &StalwartPlan,
		mail: &MailStack,
		region: &str,
	) -> Result<Self> {
		let user = StalwartBlock::ADMIN_USER.to_string();
		let admin_secret = admin_secret_name(mail)?;
		let public_origin = format!("https://{}", mail.mail_box.hostname());

		// the commissioned path: the real administrator account over the port
		// the world already reaches. No tunnel, no recovery credential, and
		// nothing on the box listening in the clear.
		if let Some(password) = ssm_ext::get(region, &admin_secret).await?
			&& let Ok(client) =
				JmapClient::connect(&public_origin, &user, &password).await
		{
			info!(
				"management over {public_origin} as the administrator account"
			);
			return Self {
				client,
				origin: public_origin,
				user,
				password,
				tunnel: None,
			}
			.xok();
		}

		// else the box is not serving its public endpoint yet, so management is
		// the plaintext port the security group deliberately does not admit,
		// reached through a forward over the box's key pair.
		let tunnel = connection
			.tunnel(provision.local_port(), StalwartProvision::MANAGEMENT_PORT)
			.await?;
		let tunnel_origin =
			format!("http://127.0.0.1:{}", provision.local_port());

		// bootstrap mode is a property of the RUNNING process rather than of
		// the disk, so it is decided by which credential answers. The endpoint
		// is waited for once, unauthenticated, so that a rejected credential
		// means "not this one" instead of "not up yet" — the difference between
		// one failed request and a ten-minute poll against the wrong password.
		wait_for_endpoint(
			&tunnel_origin,
			*provision.timeout(),
			*provision.poll(),
		)
		.await?;
		let recovery =
			read_secret(region, &mail.mail_box.admin_secret_name(&mail.stack))
				.await?;
		match JmapClient::connect(&tunnel_origin, &user, &recovery).await {
			Ok(client) => {
				Self::commission(&client, connection, plan, mail, region)
					.await?
			}
			// the recovery credential is gone, which the box only does once the
			// store is claimed: this is a claimed server that simply is not
			// serving its public endpoint yet (no certificate, or the mail
			// listeners have not bound).
			Err(_) => info!(
				"the recovery credential is retired; management over the tunnel"
			),
		}

		info!(
			"restarting {} to leave bootstrap mode",
			StalwartProvision::UNIT
		);
		connection.run_command(&restart_command()).await?;
		let password = read_secret(region, &admin_secret).await?;
		// which port the server comes back on depends on how far the store it
		// is about to load had already been commissioned, and the two cases
		// cannot be told apart before the restart. A FRESH claim boots into a
		// store that still holds the server-seeded management listener and no
		// certificate, so the tunnel is the only way in. A REBUILD recovery
		// boots into production's own store, where [`retire_listeners`] deleted
		// that listener the first time it ran and 443 holds a certificate: the
		// plaintext port never comes back, and waiting for it is a wait with no
		// end. So both are offered and whichever answers IS the channel.
		let (client, origin) = wait_for_management(
			&[&public_origin, &tunnel_origin],
			&user,
			&password,
			*provision.timeout(),
			*provision.poll(),
		)
		.await?;
		// the forward is held only while it is the channel: kept here, dropped
		// with `tunnel` going out of scope when the public endpoint won.
		let tunnel = (origin == tunnel_origin).then_some(tunnel);
		Self {
			client,
			origin,
			user,
			password,
			tunnel,
		}
		.xok()
	}

	/// Claim the data store, or recover the one state a claim cannot: a REBUILT
	/// box, whose disk is new and whose database is not, and which therefore
	/// boots into a bootstrap mode that is a lie the missing file told.
	///
	/// Called only while the server still answers to the recovery credential,
	/// ie while it is genuinely running in bootstrap mode. The caller restarts
	/// afterwards either way: the running process stays in bootstrap mode until
	/// it boots against a `config.json`.
	async fn commission(
		client: &JmapClient,
		connection: &SshConnection,
		plan: &StalwartPlan,
		mail: &MailStack,
		region: &str,
	) -> Result {
		// the file is the truth of whether a claim already happened, which is
		// the state a crash between the claim and its restart leaves behind.
		if connection
			.run_command(
				"test -f /etc/stalwart/config.json && echo yes || echo no",
			)
			.await?
			.stdout
			.xmap(|stdout| String::from_utf8_lossy(&stdout).contains("yes"))
		{
			info!("the data store is already claimed; a restart finishes it");
			return Ok(());
		}
		if let Err(err) =
			bootstrap(client, connection, plan, mail, region).await
		{
			warn!(
				"the claim was refused, so this box is being treated as a \
				rebuild onto an existing data store: {err}"
			);
			connection
				.run_command(
					"sudo -n /usr/local/bin/stalwart-secrets render-store",
				)
				.await?;
		}
		Ok(())
	}

	fn client(&self) -> &JmapClient { &self.client }

	/// Restart the unit and reconnect on the same channel: mail listeners only
	/// bind at start, so a server configured while running is still a server
	/// serving nothing.
	async fn restart(
		&mut self,
		connection: &SshConnection,
		provision: &StalwartProvision,
	) -> Result {
		connection.run_command(&restart_command()).await?;
		let origin = self.origin.clone();
		(self.client, _) = wait_for_management(
			&[&origin],
			&self.user,
			&self.password,
			*provision.timeout(),
			*provision.poll(),
		)
		.await?;
		Ok(())
	}
}

/// The parameter the server's own administrator credential is parked at, ie
/// the one the `Bootstrap` claim minted and every later run authenticates with.
fn admin_secret_name(mail: &MailStack) -> Result<String> {
	mail.serving()
		.next()
		.map(|domain| {
			AccountPlan::secret_ref(
				mail.mail_box.label(),
				StalwartBlock::ADMIN_USER,
				&domain.slug(),
			)
			.name(&mail.stack)
		})
		.ok_or_else(|| bevyhow!("no mail domain to name an administrator on"))
}

/// The restart, which also re-renders the box's secrets: `ExecStartPre` runs
/// the boot script, so the credential set the server comes back with is the one
/// the current on-disk state calls for.
fn restart_command() -> String {
	format!("sudo -n systemctl restart {}", StalwartProvision::UNIT)
}

/// Remove every listener the server seeded that the plan does not declare,
/// reporting whether anything went.
///
/// Additive-only is the rule everywhere else in this step, and this is the one
/// deliberate exception: an undeclared listener is a bound port, and a bound
/// port is not something a declaration can leave alone. The names are named
/// (rather than "anything not in the plan") so a listener somebody added on
/// purpose survives a deploy.
async fn retire_listeners(
	client: &JmapClient,
	plan: &StalwartPlan,
) -> Result<bool> {
	let declared = plan
		.listeners
		.iter()
		.filter_map(|listener| listener["name"].as_str())
		.collect::<Vec<_>>();
	let mut retired = false;
	for listener in client.list("x:NetworkListener").await? {
		let (Some(id), Some(name)) =
			(listener["id"].as_str(), listener["name"].as_str())
		else {
			continue;
		};
		if declared.contains(&name)
			|| !StalwartPlan::RETIRED_LISTENERS.contains(&name)
		{
			continue;
		}
		client.destroy("x:NetworkListener", id).await?;
		info!("retired the seeded `{name}` listener");
		retired = true;
	}
	Ok(retired)
}

/// Ask for a certificate for every domain whose declared names no stored
/// certificate covers, then wait until every domain is covered.
///
/// The server only schedules issuance itself on a domain's Manual ->
/// Automatic transition and at renewal, so a failed first order or a changed
/// SAN list would otherwise wait for an expiry that never comes. Covered means
/// no task, which is what makes a reprovision idempotent.
async fn converge_certificates(
	client: &JmapClient,
	plan: &StalwartPlan,
	domain_ids: &[String],
	timeout: Duration,
	poll: Duration,
) -> Result {
	let certificates = client.list("x:Certificate").await?;
	let mut waiting = Vec::new();
	for (domain, domain_id) in plan.domains.iter().zip(domain_ids) {
		if certificate_covers(&certificates, &domain.certificate_names) {
			continue;
		}
		client
			.create(
				"x:Task",
				&json!({ "@type": "AcmeRenewal", "domainId": domain_id }),
			)
			.await?;
		info!("{}: certificate issuance scheduled", domain.name);
		waiting.push(domain);
	}
	if waiting.is_empty() {
		return Ok(());
	}
	let attempts = (timeout.as_secs() / poll.as_secs().max(1)).max(1);
	for _ in 0..attempts {
		time_ext::sleep(poll).await;
		let certificates = client.list("x:Certificate").await?;
		waiting.retain(|domain| {
			!certificate_covers(&certificates, &domain.certificate_names)
		});
		if waiting.is_empty() {
			info!("every domain's certificate is issued and stored");
			return Ok(());
		}
	}
	bevybail!(
		"certificates never arrived for: {} (the ACME order may be failing; \
		the server log names the rejected hostname)",
		waiting
			.iter()
			.map(|domain| domain.name.as_str())
			.collect::<Vec<_>>()
			.join(", ")
	)
}

/// Claim a fresh data store through the `Bootstrap` singleton, or do nothing.
///
/// A `0.16` server whose `config.json` is absent boots in bootstrap mode, where
/// the ONE settable object is `Bootstrap`: patching its singleton hands the
/// server every store at once, and the server then creates the SQL tables,
/// writes `config.json` itself, creates the default domain and an
/// `admin@<domain>` account, and leaves bootstrap mode — all before this
/// returns. The probe is `x:Bootstrap/get`, which answers with its
/// singleton exactly while bootstrap mode lasts, so a rerun against a claimed
/// server skips the whole step.
///
/// The data store settings are read off the BOX's own template
/// (`config.json.template`, terraform-rendered) rather than re-composed here,
/// so the host the claim names and the host the box re-renders at every later
/// start are one string. Only the secret is filled in, from the same parameter
/// the box reads.
///
/// The server MINTS the admin account's password and returns it in the set
/// response, this one time; it is parked at the same parameter composition
/// every other account's credential uses, overwriting any stale value from a
/// previous deployment's store (a claim only ever succeeds on a blank one).
///
/// Errors rather than recovering, since the caller is the one place that can
/// tell a refused claim on a BLANK store (a real failure) from one on a store
/// this box was rebuilt onto.
async fn bootstrap(
	client: &JmapClient,
	connection: &SshConnection,
	plan: &StalwartPlan,
	mail: &MailStack,
	region: &str,
) -> Result {
	let stack = &mail.stack;
	let template = connection
		.run_command("cat /etc/stalwart/config.json.template")
		.await?;
	let template = String::from_utf8(template.stdout)?;
	let mut data_store: Value =
		serde_json::from_str(template.trim()).map_err(|err| {
			bevyhow!("the box's config.json.template is not json: {err}")
		})?;
	data_store["authSecret"] = json!({
		"@type": "Value",
		"secret": read_secret(region, &mail.mail_box.database().secret_name(stack)).await?,
	});
	let domain = mail
		.serving()
		.next()
		.ok_or_else(|| bevyhow!("no mail domain to bootstrap with"))?;

	info!("claiming the data store (creates tables, may take a minute)...");
	let claim = client
		.update_returning(
			"x:Bootstrap",
			JmapClient::SINGLETON_ID,
			&json!({
				"serverHostname": plan.hostname,
				"defaultDomain": domain.domain(),
				// provision owns certificates and DKIM: ACME converges as its own
				// object below, and outbound is SES-signed until the sovereign
				// selector lands
				"requestTlsCertificate": false,
				"generateDkimKeys": false,
				"dataStore": data_store,
				"blobStore": mail.mail_box.blob_store_config(stack),
				// `Default` IS the data store, which is the declared shape: postgres
				// carries search and ephemera, S3 carries blobs
				"searchStore": { "@type": "Default" },
				"inMemoryStore": { "@type": "Default" },
				"directory": { "@type": "Internal" },
			}),
		)
		.await;
	let updated = claim?;

	let secret_name = AccountPlan::secret_ref(
		mail.mail_box.label(),
		StalwartBlock::ADMIN_USER,
		&domain.slug(),
	)
	.name(stack);
	match updated["secret"].as_str() {
		Some(secret) => {
			ssm_ext::overwrite(region, &secret_name, secret).await?;
			info!("parked the bootstrap admin credential at {secret_name}");
		}
		None => warn!(
			"the bootstrap claim returned no admin credential; expected one at \
			`secret` in the set response"
		),
	}
	info!("data store claimed, {} created", domain.domain());
	Ok(())
}

/// Poll `origin` until the management endpoint answers at all, whatever it
/// answers with.
///
/// Unauthenticated on purpose: it separates "the server is not up" from "that
/// credential is wrong", which are the two things a bootstrap-mode box looks
/// identical from the outside for.
async fn wait_for_endpoint(
	origin: &str,
	timeout: Duration,
	poll: Duration,
) -> Result {
	let url = format!("{origin}{}", JmapClient::SESSION_PATH);
	let attempts = (timeout.as_secs() / poll.as_secs().max(1)).max(1);
	for attempt in 1..=attempts {
		if Request::get(&url).send().await.is_ok() {
			info!("management endpoint answering after {attempt} attempt(s)");
			return Ok(());
		}
		time_ext::sleep(poll).await;
	}
	bevybail!("the management endpoint at {origin} never answered")
}

/// Converge the server onto `plan`, in dependency order. Returns each domain's
/// id, in the plan's own order, for the certificate step that runs after the
/// restart.
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
) -> Result<Vec<String>> {
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

	let mut domain_ids = Vec::new();
	for domain in &plan.domains {
		let domain_id =
			converge(client, "x:Domain", &["name"], &domain.object(&acme))
				.await?;
		converge_dkim(client, domain, &domain_id, region, stack).await?;
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
		domain_ids.push(domain_id);
	}

	let default_domain = domain_ids
		.first()
		.ok_or_else(|| bevyhow!("the plan declared no domain to default to"))?;
	client
		.update_singleton(
			"x:SystemSettings",
			&plan.system_settings(default_domain),
		)
		.await?;
	info!("hostname is {}", plan.hostname);
	Ok(domain_ids)
}

/// Whether any stored certificate covers every one of `names`.
///
/// A certificate's `subjectAlternativeNames` arrives in the registry's `Map`
/// wire shape, ie the names are the KEYS.
fn certificate_covers(certificates: &[Value], names: &[String]) -> bool {
	certificates.iter().any(|certificate| {
		let Some(sans) = certificate["subjectAlternativeNames"].as_object()
		else {
			return false;
		};
		names.iter().all(|name| sans.contains_key(name))
	})
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

/// Hand the domain the sovereign signing key whose public half its selector
/// record already carries.
///
/// The key comes from parameter store rather than from the server, which is the
/// whole point of the arrangement: the record was published by the apply that
/// read the same parameter, so the selector the world resolves and the key the
/// server signs with cannot disagree. A server-generated key would have to be
/// read back and published in a second apply, with a window in between where
/// mail is signed by a selector nothing answers for.
///
/// The credential is written on creation only, for the same reason an account's
/// is: a key rotated under a published selector signs mail no verifier can
/// check until DNS catches up. Rotation is a second selector beside this one.
async fn converge_dkim(
	client: &JmapClient,
	domain: &DomainPlan,
	domain_id: &str,
	region: &str,
	stack: &ResolvedStack,
) -> Result {
	let name = domain.dkim_secret.name(stack);
	let private_key = ssm_ext::get(region, &name).await?.ok_or_else(|| {
		bevyhow!(
			"no dkim key at {name}: <EnsureDkimKey/> mints it and the apply \
			publishes its public half, so both run before this step"
		)
	})?;
	// matched WITHOUT the key, so an existing signature never shows a diff on
	// the one property that must not be patched
	let existing = client.list("x:DkimSignature").await?;
	match plan_converge(
		&existing,
		&["selector", "domainId"],
		&domain.dkim_object(domain_id, None),
	)? {
		Converge::Create => {
			client
				.create(
					"x:DkimSignature",
					&domain.dkim_object(domain_id, Some(&private_key)),
				)
				.await?;
			info!(
				"{} signs with its own `{}` selector",
				domain.name,
				MailDomainBlock::DKIM_SELECTOR
			);
		}
		Converge::Unchanged(_) => {}
		Converge::Patch(id, patch) => {
			client.update("x:DkimSignature", &id, &patch).await?
		}
	}
	Ok(())
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



/// Poll the management endpoint until it authenticates, which is also the check
/// that the box finished booting and opened its data store. Returns the client
/// and the origin that answered.
///
/// Takes every origin the server might come back on rather than the one it is
/// expected to, because after a restart that is genuinely not known: a box
/// whose store has been through [`retire_listeners`] has no plaintext
/// management port to come back on, and a box whose store has not been claimed
/// has no certificate to serve 443 with. Each round tries them in order, so a
/// caller passing one origin gets exactly the old behaviour and a caller
/// passing two gets whichever is real, at the cost of one failed request.
async fn wait_for_management(
	origins: &[&str],
	user: &str,
	password: &str,
	timeout: Duration,
	poll: Duration,
) -> Result<(JmapClient, String)> {
	let attempts = (timeout.as_secs() / poll.as_secs().max(1)).max(1);
	let mut last = None;
	for attempt in 1..=attempts {
		for origin in origins {
			match JmapClient::connect(*origin, user, password).await {
				Ok(client) => {
					info!(
						"management endpoint {origin} ready after {attempt} \
						attempt(s)"
					);
					return Ok((client, origin.to_string()));
				}
				Err(err) => last = Some(err),
			}
		}
		time_ext::sleep(poll).await;
	}
	bevybail!(
		"the management endpoint never answered on any of {}: {}",
		origins.join(", "),
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

	/// Every origin offered is tried in every round, which is the property the
	/// rebuild-recovery path rests on: the restarted box comes back on ONE of
	/// the two channels and which one is not knowable beforehand. Before this,
	/// the recovery waited on the tunnel alone and a box whose store had
	/// already retired the plaintext management listener never answered it —
	/// a wait with no end against a server that was serving mail perfectly.
	#[beet_core::test]
	async fn every_offered_origin_is_tried() {
		// two ports nothing is listening on, so each attempt is a refused
		// connection rather than a wait, and the whole test is milliseconds
		let Err(err) = wait_for_management(
			&["http://127.0.0.1:1", "http://127.0.0.1:2"],
			"admin",
			"hunter2",
			Duration::from_secs(1),
			Duration::from_secs(1),
		)
		.await
		else {
			panic!("nothing is listening, so nothing can have connected");
		};
		err.to_string()
			.xpect_contains("http://127.0.0.1:1")
			.xpect_contains("http://127.0.0.1:2");
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
