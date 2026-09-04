//! The rehearsal that turns a backup into a restore.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::json;

/// `<MailRestoreDrill/>` — restore another stage's newest database dump into
/// THIS stage's mail box, so the mailboxes that come back can be probed.
///
/// A backup nobody has restored is a hypothesis. The nightly dump is written by
/// a timer on the box and lands in a bucket where it is indistinguishable from
/// a file of zeroes, and every property that matters — that it is complete,
/// that the client can read it, that the schema comes back, that the server
/// opens the store afterwards — is only observable by doing it. So the drill is
/// a route rather than a runbook, and the assertion is the ordinary
/// [`MailProbe`] running against the restored stage: mail flows, or the backup
/// was not one.
///
/// It is a whole STAGE, not a spare database: a drill deploy stands up its own
/// network, database and box, restores production's dump into them, and is
/// destroyed after. That also makes this the rehearsal for the region move
/// decision 1 keeps open, since the steps are the same ones.
///
/// The assertion is deliberately made HERE rather than by a [`MailProbe`]
/// beside it. A restored store carries the SOURCE stage's domains and accounts,
/// so the drill box now serves `probe@<source domain>` while that domain's `MX`
/// still points at the source box: a probe's inbound leg would be answered by
/// production and pass without the drill having restored anything. What is
/// genuinely provable is what this asserts — that a restored account
/// authenticates against the DRILL box and its mailbox is readable — and that
/// is what "the backup came back" means.
///
/// The one thing this action will not do is run against the stage it is
/// restoring FROM. A `pg_restore --clean` into the live mail database is not a
/// drill, it is the incident, so the stages are compared and a match fails
/// before anything is downloaded.
#[derive(Debug, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(MailRestoreDrillAction)]
pub struct MailRestoreDrill {
	/// The stage whose backups are restored, ie the one being proven
	/// recoverable.
	source_stage: SmolStr,
	/// The mail domain the restored account belongs to, ie one the SOURCE
	/// stage serves.
	///
	/// Named rather than read off this stack, because the drill stack does not
	/// declare it and must not: an SES identity is account-global, so a second
	/// stack declaring `stalwart.beetmash.com` fails its apply against the
	/// identity the live one already owns. The drill therefore serves a
	/// domain of its own — which the restore then REPLACES with the source's,
	/// and the account this signs in as is one of those.
	source_domain: SmolStr,
	/// The private half of the key pair the box imported, as
	/// [`StalwartProvision`] takes it.
	ssh_key: SmolStr,
	/// The restored mailbox the assertion signs in as, by localpart. The probe
	/// mailbox by default, since it is the one account whose credential exists
	/// to be used by a deploy step.
	mailbox: SmolStr,
	/// How long to wait for the restarted server to accept a restored
	/// credential.
	timeout: Duration,
}

impl Default for MailRestoreDrill {
	fn default() -> Self {
		Self {
			source_stage: Self::SOURCE_STAGE.into(),
			source_domain: SmolStr::default(),
			ssh_key: StalwartProvision::SSH_KEY.into(),
			mailbox: "probe".into(),
			timeout: Duration::from_secs(300),
		}
	}
}

impl MailRestoreDrill {
	/// The stage a drill proves by default, ie the one carrying real mail.
	pub const SOURCE_STAGE: &'static str = "prod";

	/// Where the dump is staged on the box. Under the service account's own
	/// directory rather than `/tmp`, so it inherits the same ownership as
	/// everything else the box holds and is removed on the same line.
	pub const REMOTE_PATH: &'static str = "/var/lib/stalwart/restore.dump";

	/// Where the dump lands off the wire, which is NOT where it is restored
	/// from.
	///
	/// `scp` arrives as the login user and `/var/lib/stalwart` is `0700
	/// stalwart:stalwart` — correct for a directory holding mail, and it means
	/// a dump has to be INSTALLED into it rather than delivered. So the file
	/// crosses into the login user's own directory and is moved across with
	/// the service account's ownership, which is also the only moment either
	/// end of the transfer is readable by anything but root.
	pub fn upload_path() -> String {
		format!("/home/{}/mail-restore.dump", StalwartProvision::SSH_USER)
	}

	/// Put the uploaded dump where the service account can read it, and leave
	/// nothing behind on the login user's side.
	fn stage_command() -> String {
		format!(
			"sudo -n install -o stalwart -g stalwart -m 0600 '{upload}' \
			'{remote}' && rm -f '{upload}'",
			upload = Self::upload_path(),
			remote = Self::REMOTE_PATH,
		)
	}
}

/// Finds the newest dump, carries it to the box and restores it, leaving the
/// server running against the restored store.
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn MailRestoreDrillAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let drill = cx
		.caller
		.get_cloned::<MailRestoreDrill>()
		.await
		.unwrap_or_default();
	let mail = cx.caller.with_world(MailStack::resolve).await??;
	let stage = mail.stack.stage().clone();
	if &stage == drill.source_stage() {
		bevybail!(
			"this drill would restore {stage}'s own backup over {stage}'s live \
			database. Deploy the stack to a throwaway stage and run it there: \
			`--stage=drill`"
		);
	}
	if drill.source_domain().is_empty() {
		bevybail!(
			"no source_domain: the drill signs in as an account the RESTORE \
			created, which belongs to a domain the '{}' stage serves and this \
			stack deliberately does not declare",
			drill.source_stage()
		);
	}

	// the SOURCE stage's bucket, which is the same declaration resolved against
	// a different stage: the one place the two stacks touch.
	let source = mail.stack.clone().with_stage(drill.source_stage().clone());
	let bucket = source.resource_name(mail.mail_box.backup_bucket().clone());
	let region = mail.stack.region().clone();
	let prefix = format!(
		"{}/{}",
		StalwartBlock::BACKUP_PREFIX,
		mail.mail_box.db_name()
	);
	let key = newest_dump(&region, &bucket, &prefix).await?;
	info!("restoring s3://{bucket}/{key} into the {stage} stage");

	let local = mail.project.work_dir().join("mail-restore.dump");
	ChildProcess::new("aws")
		.without_env("AWS_PROFILE")
		.with_args([
			"s3".to_string(),
			"cp".to_string(),
			format!("s3://{bucket}/{key}"),
			local.display().to_string(),
			"--region".to_string(),
			region.to_string(),
		])
		.run_async()
		.await?;

	let connection = SshConnection {
		host: mail.public_ip().await?,
		user: StalwartProvision::SSH_USER.to_string(),
		port: 22,
		key_path: StalwartProvision::key_path(drill.ssh_key())?,
	};
	connection
		.wait_for_ready(Duration::from_secs(300), Duration::from_secs(5))
		.await?;
	connection
		.scp_to(local.as_ref(), &MailRestoreDrill::upload_path())
		.await?;
	fs_ext::remove(&local).ok();
	connection
		.run_command(&MailRestoreDrill::stage_command())
		.await?;

	// the server holds the store open and caches most of it, so it is stopped
	// around the restore rather than asked to notice: a `pg_restore --clean`
	// under a live Stalwart drops tables it is mid-query on.
	info!("stopping {} for the restore", StalwartProvision::UNIT);
	connection
		.run_command(&format!(
			"sudo -n systemctl stop {}",
			StalwartProvision::UNIT
		))
		.await?;
	let output = connection
		.run_command(&restore_command(&mail, &mail.database_host().await?))
		.await?;
	info!(
		"pg_restore: {}",
		String::from_utf8_lossy(&output.stdout).trim()
	);
	connection
		.run_command(&format!(
			"sudo -n systemctl start {}",
			StalwartProvision::UNIT
		))
		.await?;

	assert_restored(
		&mail,
		&source,
		drill.source_domain(),
		drill.mailbox(),
		*drill.timeout(),
	)
	.await?;
	info!(
		"the {stage} stage serves {}'s restored mail: the backup is one",
		drill.source_stage()
	);
	Pass(cx.input).xok()
}

/// Sign in to the DRILL box as an account that only exists because the restore
/// worked, and read its mailbox.
///
/// Both halves of that sentence are the assertion. The credential is the SOURCE
/// stage's parameter, so authenticating at all proves the restored store
/// carries the source's accounts and password hashes; reading the mailbox
/// proves the message metadata came back rather than just the schema.
///
/// The address is the SOURCE's, not this stack's. Before the restore the drill
/// box served its own throwaway domain and its own empty accounts; a
/// `pg_restore --clean` replaced all of it, so the only account there now is
/// one this stack never declared.
///
/// Which is also why the CONNECTION is made the awkward way, and the
/// awkwardness is worth stating because it is a property of the design rather
/// than of this code. Stalwart `0.16` keeps its configuration in the data
/// store, so a restore carries the source's whole identity — its hostname, its
/// domains, its listeners and its certificates all live in the database the
/// mail lives in. The moment the restore lands, this box stops answering to
/// `mail-drill.beetmash.com` and starts serving PRODUCTION's certificate for
/// production's names, and there is no name that both resolves here and is
/// covered by the certificate this box now holds. So the address is forced
/// rather than resolved: `curl --resolve` dials the drill's own IP while
/// verifying the certificate against `autoconfig.<source domain>`, which
/// [`StalwartPlan`] puts on every certificate it issues and which is the one
/// such name derivable from what this stack declares.
///
/// Verification stays ON, and passing it is part of the assertion: a box that
/// had restored nothing could not present that certificate. Resolving the name
/// normally would be the opposite of a test — it would reach the live box and
/// pass without this stage having done anything at all.
async fn assert_restored(
	mail: &MailStack,
	source: &ResolvedStack,
	source_domain: &str,
	localpart: &str,
	timeout: Duration,
) -> Result {
	let address = format!("{localpart}@{source_domain}");
	let secret = AccountPlan::secret_ref(
		mail.mail_box.label(),
		localpart,
		&MailDomainBlock::slug_of(source_domain),
	)
	.name(source);
	let password = ssm_ext::get(&mail.stack.region(), &secret)
		.await?
		.ok_or_else(|| {
			bevyhow!(
				"no credential at {secret}: the drill authenticates as one of \
				the SOURCE stage's accounts, so its parameters must still exist"
			)
		})?;

	let host =
		format!("{}.{source_domain}", MailDomainBlock::AUTOCONFIG_LABELS[0]);
	let ip = mail.public_ip().await?;
	let poll = Duration::from_secs(5);
	let attempts = (timeout.as_secs() / poll.as_secs()).max(1);
	let mut last = None;
	for _ in 0..attempts {
		match read_mailboxes(&host, &ip, &address, &password).await {
			Ok(0) => bevybail!(
				"{address} authenticated against the restored store but holds \
				no mailboxes, so the schema came back and the data did not"
			),
			Ok(mailboxes) => {
				info!(
					"{address} signed in on {ip} behind {host}'s restored \
					certificate and holds {mailboxes} mailbox(es)"
				);
				return Ok(());
			}
			Err(err) => {
				last = Some(err);
				time_ext::sleep(poll).await;
			}
		}
	}
	bevybail!(
		"{address} never authenticated against the restored store: {}",
		last.map(|err| err.to_string()).unwrap_or_default()
	)
}

/// One JMAP session and one `Mailbox/get` against the restored server, over a
/// forced address.
///
/// `curl` rather than [`JmapClient`] for the one reason [`MailProbe`] reaches
/// for it too: the request needs something the client cannot express — here an
/// address that overrides DNS while the certificate is still verified against
/// the name. The password rides `--user`, which is exactly the case
/// [`ChildProcess::with_secret`] exists for.
async fn read_mailboxes(
	host: &str,
	ip: &str,
	address: &str,
	password: &str,
) -> Result<usize> {
	let session: serde_json::Value = serde_json::from_str(
		&jmap_curl(host, ip, address, password, JmapClient::SESSION_PATH, None)
			.await?,
	)?;
	let account = session["primaryAccounts"][JmapClient::MAIL_CAPABILITY]
		.as_str()
		.ok_or_else(|| {
			bevyhow!(
				"the restored session names no primary mail account for \
				{address}, so it authenticated as something other than a mailbox"
			)
		})?
		.to_string();
	let api_path = session["apiUrl"]
		.as_str()
		.map(JmapClient::url_to_path)
		.ok_or_else(|| bevyhow!("the restored session carried no apiUrl"))?;
	let body = json!({
		"using": ["urn:ietf:params:jmap:core", JmapClient::MAIL_CAPABILITY],
		"methodCalls": [
			["Mailbox/get", { "accountId": account }, "0"]
		]
	});
	let response: serde_json::Value = serde_json::from_str(
		&jmap_curl(
			host,
			ip,
			address,
			password,
			&api_path,
			Some(&body.to_string()),
		)
		.await?,
	)?;
	response["methodResponses"][0][1]["list"]
		.as_array()
		.map(Vec::len)
		.unwrap_or_default()
		.xok()
}

/// One authenticated request at `path`, dialled at `ip` and verified against
/// `host`.
async fn jmap_curl(
	host: &str,
	ip: &str,
	address: &str,
	password: &str,
	path: &str,
	body: Option<&str>,
) -> Result<String> {
	let mut args = vec![
		"--silent".to_string(),
		"--show-error".to_string(),
		"--fail".to_string(),
		// the whole trick: this address, that name's certificate
		"--resolve".to_string(),
		format!("{host}:443:{ip}"),
		"--user".to_string(),
		format!("{address}:{password}"),
	];
	if let Some(body) = body {
		args.extend([
			"--header".to_string(),
			"content-type: application/json".to_string(),
			"--data".to_string(),
			body.to_string(),
		]);
	}
	args.push(format!("https://{host}{path}"));
	ChildProcess::new("curl")
		.with_args(args)
		.with_secret(password)
		.run_async_stdout()
		.await
}

/// The newest object under `prefix`, by last-modified rather than by name.
///
/// The keys are date-ordered, so sorting by name would agree today — and stop
/// agreeing the moment a dump is copied, re-uploaded or restored from an
/// archive tier, which are exactly the circumstances a drill runs in.
async fn newest_dump(
	region: &str,
	bucket: &str,
	prefix: &str,
) -> Result<String> {
	let key = ChildProcess::new("aws")
		.without_env("AWS_PROFILE")
		.with_args([
			"s3api",
			"list-objects-v2",
			"--bucket",
			bucket,
			"--prefix",
			prefix,
			"--query",
			"sort_by(Contents,&LastModified)[-1].Key",
			"--output",
			"text",
			"--region",
			region,
		])
		.run_async_stdout()
		.await?
		.trim()
		.to_string();
	match key.is_empty() || key == "None" {
		true => bevybail!(
			"s3://{bucket}/{prefix} holds no dump, so there is nothing to \
			restore: the box's backup timer is what fills it, and `systemctl \
			list-timers` on the box is where to look"
		),
		false => key.xok(),
	}
}

/// The restore itself, run on the box because the database is in a private
/// subnet and reachable from nowhere else.
///
/// The box reads its own stage's database credential from parameter store, so
/// no secret rides this command line. `PGSSLROOTCERT=system` rides beside
/// `verify-full` because libpq looks for `~/.postgresql/root.crt` rather than
/// the OS trust store the boot script populated, and without it the restore
/// fails on a CA the box demonstrably trusts. `--clean --if-exists` because a drill
/// stage has already been provisioned with its own empty mailboxes, and
/// `--no-owner --no-acl` because the dump's roles are the source stage's.
fn restore_command(mail: &MailStack, host: &str) -> String {
	let secret = mail.database.secret_name(&mail.stack);
	format!(
		"sudo -n -u stalwart env \
		PGPASSWORD=\"$(aws ssm get-parameter --region '{region}' --name '{secret}' --with-decryption --query Parameter.Value --output text)\" \
		PGSSLMODE=verify-full PGSSLROOTCERT=system \
		pg_restore --host '{host}' --port {port} --username '{user}' \
		--dbname '{database}' --clean --if-exists --no-owner --no-acl \
		'{path}'; sudo -n rm -f '{path}'",
		region = mail.stack.region(),
		host = host,
		port = RdsPostgresBlock::PORT,
		user = mail.mail_box.db_user(),
		database = mail.mail_box.db_name(),
		path = MailRestoreDrill::REMOTE_PATH,
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The credential the drill authenticates with belongs to the SOURCE: the
	/// source stage's parameter prefix, and the source DOMAIN's slug.
	///
	/// Both halves are easy to get wrong in the same direction, and both fail
	/// as "no credential at .." three steps after an hour-long deploy. The
	/// stack under the drill declares a throwaway domain of its own, which the
	/// restore then deletes, so reading either end off it would name an
	/// account that no longer exists.
	#[beet_core::test]
	fn the_drill_reads_the_source_stage_and_the_source_domain() {
		let (stack, _deployment, _dir) = ResolvedStack::default_local();
		let source = stack.clone().with_stage("prod");
		AccountPlan::secret_ref(
			"mail",
			"probe",
			&MailDomainBlock::slug_of("stalwart.beetmash.com"),
		)
		.name(&source)
		.xpect_contains("/prod/")
		.xpect_contains("mail-account-probe-at-stalwart-beetmash-com");
	}

	/// The dump is delivered to a path the login user can write and restored
	/// from one only the service account can read. They are not the same path,
	/// and the reason is a directory mode rather than a preference: an scp
	/// straight into the mail store's directory fails with "Permission denied"
	/// after the dump has already crossed the wire.
	#[beet_core::test]
	fn the_dump_is_installed_rather_than_delivered() {
		let upload = MailRestoreDrill::upload_path();
		upload.as_str().xpect_contains(StalwartProvision::SSH_USER);
		(upload.as_str() == MailRestoreDrill::REMOTE_PATH).xpect_false();
		MailRestoreDrill::stage_command()
			.as_str()
			.xpect_contains("-o stalwart -g stalwart")
			.xpect_contains(MailRestoreDrill::REMOTE_PATH);
	}

	/// The restore reaches the database by name. `RdsPostgresBlock::host`
	/// composes a terraform reference, which is the right value inside a config
	/// file and a literal `${aws_db_instance..}` over ssh, so this command is
	/// built from the apply's OUTPUT instead.
	#[beet_core::test]
	fn the_restore_names_a_host_rather_than_a_terraform_reference() {
		let (stack, deployment, _dir) = ResolvedStack::default_local();
		let mail_box = StalwartBlock::new("mail", "mail.beetmash.com")
			.with_db_name("mail");
		let command = restore_command(
			&MailStack {
				project: terra::Project::new(
					stack.clone(),
					deployment,
					default(),
				),
				stack,
				mail_box,
				database: RdsPostgresBlock::new("db"),
				domains: Vec::new(),
				relays: default(),
			},
			"db.example.ap-southeast-2.rds.amazonaws.com",
		);
		command
			.as_str()
			.xpect_contains(
				"--host 'db.example.ap-southeast-2.rds.amazonaws.com'",
			)
			.xnot()
			.xpect_contains("${aws_db_instance");
	}
}
