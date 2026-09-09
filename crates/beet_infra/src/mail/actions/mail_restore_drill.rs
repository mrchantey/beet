//! The rehearsal that turns a backup into a restore.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;
use serde_json::json;




impl MailRestoreDrill {
	/// The stage a drill proves by default, ie the one carrying real mail.
	pub const SOURCE_STAGE: &'static str = "prod";


	/// Where the snapshot is staged on the box. Under the service account's own
	/// directory rather than `/tmp`, so it inherits the same ownership as
	/// everything else the box holds.
	pub const REMOTE_PATH: &'static str = "/var/lib/stalwart/restore.db";

	/// Where the snapshot lands off the wire, which is not where it is restored
	/// from.
	///
	/// `scp` arrives as the login user and `/var/lib/stalwart` is `0700
	/// stalwart:stalwart` — correct for a directory holding mail, and it means
	/// a snapshot has to be installed into it rather than delivered. So the file
	/// crosses into the login user's own directory and is moved across with the
	/// service account's ownership.
	pub fn upload_path() -> String {
		format!("/home/{}/mail-restore.db", StalwartProvision::SSH_USER)
	}

	/// Put the uploaded snapshot where the service account can read it, and leave
	/// nothing behind on the login user's side.
	fn stage_command() -> String {
		format!(
			"sudo -n install -o stalwart -g stalwart -m 0600 '{upload}' \
			'{remote}' && rm -f '{upload}'",
			upload = Self::upload_path(),
			remote = Self::REMOTE_PATH,
		)
	}

	/// Verify the staged file before stopping a healthy server.
	fn integrity_command() -> String {
		format!(
			"result=\"$(sudo -n -u stalwart sqlite3 '{path}' 'PRAGMA integrity_check;')\"; \
			[ \"$result\" = ok ] || {{ echo \"SQLite integrity check failed for restored snapshot: $result\" >&2; exit 1; }}",
			path = Self::REMOTE_PATH,
		)
	}

	/// Atomically replace the stopped server's main database and discard sidecars
	/// belonging to the database that was replaced.
	fn replace_command() -> String {
		format!(
			"sudo -n -u stalwart mv -f '{restore}' '{database}' && \
			sudo -n -u stalwart rm -f '{database}-wal' '{database}-shm'",
			restore = Self::REMOTE_PATH,
			database = StalwartBlock::DATABASE_PATH,
		)
	}
}

/// Finds the newest snapshot, carries it to the box and restores it, leaving
/// the server running against the restored store.
/// `<MailRestoreDrill/>` — restore another stage's newest SQLite snapshot into
/// THIS stage's mail box, so the mailboxes that come back can be probed.
///
/// A backup nobody has restored is a hypothesis. The nightly snapshot is written
/// by a timer on the box and lands in a bucket where it is indistinguishable from
/// a file of zeroes, and every property that matters — that it is complete,
/// readable, internally consistent, replaceable and accepted by Stalwart — is
/// only observable by doing it. So the drill is
/// a route rather than a runbook, and the assertion is the ordinary
/// [`MailProbe`] running against the restored stage: mail flows, or the backup
/// was not one.
///
/// It is a whole STAGE, not a spare file: a drill deploy stands up its own box
/// and persistent volume, restores production's snapshot into them, and is
/// destroyed after.
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
/// restoring FROM. Replacing the live stage's SQLite file is not a drill, it is
/// the incident, so the stages are compared and a match fails before anything
/// is downloaded.
#[action]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
pub async fn MailRestoreDrill(
	/// The stage whose backups are restored, ie the one being proven
	/// recoverable.
	#[field(default = Self::SOURCE_STAGE)]
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
	#[field]
	source_domain: SmolStr,
	/// The private half of the key pair the box imported, as
	/// [`StalwartProvision`] takes it.
	#[field(default = StalwartProvision::SSH_KEY)]
	ssh_key: SmolStr,
	/// The restored mailbox the assertion signs in as, by localpart. The probe
	/// mailbox by default, since it is the one account whose credential exists
	/// to be used by a deploy step.
	#[field]
	mailbox: SmolStr,
	/// How long to wait for the restarted server to accept a restored
	/// credential.
	#[field(default = Duration::from_secs(300))]
	timeout: Duration,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let mail = cx.caller.with_world(MailStack::resolve).await??;
	let stage = mail.stack.stage().clone();
	if stage == source_stage {
		bevybail!(
			"this drill would restore {stage}'s own backup over {stage}'s live \
			SQLite database. Deploy to a throwaway stage and run it there: \
			`--stage=drill`"
		);
	}
	if source_domain.is_empty() {
		bevybail!(
			"no source_domain: the drill signs in as an account the RESTORE \
			created, which belongs to a domain the '{}' stage serves and this \
			stack deliberately does not declare",
			source_stage
		);
	}
	if mail.mail_box.backup_bucket().is_empty() {
		bevybail!(
			"mail box '{}' declares no backup bucket, so there is no snapshot \
			archive to restore from",
			mail.mail_box.label()
		);
	}

	// the SOURCE stage's bucket, which is the same declaration resolved against
	// a different stage: the one place the two stacks touch.
	let source = mail.stack.clone().with_stage(source_stage.clone());
	let bucket = source.resource_name(mail.mail_box.backup_bucket().clone());
	let region = mail.stack.region().clone();
	let prefix = format!("{}/", StalwartBlock::BACKUP_PREFIX);
	let key = newest_snapshot(&region, &bucket, &prefix).await?;
	info!("restoring s3://{bucket}/{key} into the {stage} stage");

	let local = mail.project.work_dir().join("mail-restore.db");
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
		key_path: StalwartProvision::key_path(&ssh_key)?,
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

	// Reject a damaged object while the current server is still healthy. Only a
	// verified standalone database is allowed to trigger downtime.
	connection
		.run_command(&MailRestoreDrill::integrity_command())
		.await?;
	info!("SQLite integrity check passed for {key}");

	// Stalwart owns the live database and may have uncheckpointed sidecars. Stop
	// it, replace the main file, remove the old WAL/SHM pair, then start against
	// exactly the verified snapshot.
	info!("stopping {} for the restore", StalwartProvision::UNIT);
	connection
		.run_command(&format!(
			"sudo -n systemctl stop {}",
			StalwartProvision::UNIT
		))
		.await?;
	connection
		.run_command(&MailRestoreDrill::replace_command())
		.await?;
	connection
		.run_command(&format!(
			"sudo -n systemctl start {}",
			StalwartProvision::UNIT
		))
		.await?;

	assert_restored(&mail, &source, &source_domain, &mailbox, timeout).await?;
	info!(
		"the {stage} stage serves {}'s restored mail: the backup is one",
		source_stage
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
/// replacing the SQLite database replaced all of it, so the only account there
/// now is one this stack never declared.
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

/// The newest `.db` object under `prefix`, by last-modified rather than name.
///
/// The keys are date-ordered, so sorting by name would agree today — and stop
/// agreeing the moment a snapshot is copied, re-uploaded or restored from an
/// archive tier, which are exactly the circumstances a drill runs in. Filtering
/// first prevents a marker or unrelated object under `sqlite/` winning.
async fn newest_snapshot(
	region: &str,
	bucket: &str,
	prefix: &str,
) -> Result<String> {
	ChildProcess::new("aws")
		.without_env("AWS_PROFILE")
		.with_args([
			"s3api",
			"list-objects-v2",
			"--bucket",
			bucket,
			"--prefix",
			prefix,
			"--output",
			"json",
			"--region",
			region,
		])
		.run_async_stdout()
		.await?
		.xmap(|body| newest_snapshot_key(&body, bucket, prefix))
}

/// Select the newest valid snapshot from an S3 listing.
fn newest_snapshot_key(
	body: &str,
	bucket: &str,
	prefix: &str,
) -> Result<String> {
	let listing: Value = serde_json::from_str(body)?;
	let mut snapshots = listing["Contents"]
		.as_array()
		.into_iter()
		.flatten()
		.filter_map(|object| {
			let key = object["Key"].as_str()?;
			let modified = object["LastModified"].as_str()?;
			(key.starts_with(prefix) && key.ends_with(".db"))
				.then(|| (modified, key))
		});
	let Some(first) = snapshots.next() else {
		bevybail!(
			"s3://{bucket}/{prefix} holds no SQLite .db snapshot, so there is \
			nothing to restore: the box's backup timer is what fills it, and \
			`systemctl list-timers` on the box is where to look"
		);
	};
	snapshots
		.fold(first, |newest, candidate| match candidate.0 > newest.0 {
			true => candidate,
			false => newest,
		})
		.1
		.to_string()
		.xok()
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

	/// The snapshot is delivered to a path the login user can write and restored
	/// from one only the service account can read. They are not the same path,
	/// because an scp straight into the mail store fails with permission denied.
	#[beet_core::test]
	fn snapshot_is_installed_rather_than_delivered() {
		let upload = MailRestoreDrill::upload_path();
		upload.as_str().xpect_contains(StalwartProvision::SSH_USER);
		(upload.as_str() == MailRestoreDrill::REMOTE_PATH).xpect_false();
		MailRestoreDrill::stage_command()
			.as_str()
			.xpect_contains("-o stalwart -g stalwart")
			.xpect_contains(MailRestoreDrill::REMOTE_PATH);
	}

	#[beet_core::test]
	fn newest_snapshot_selects_only_db_files() {
		newest_snapshot_key(
			r#"{"Contents":[
				{"Key":"sqlite/2026/09/05/old.db","LastModified":"2026-09-05T14:30:00Z"},
				{"Key":"sqlite/notes.txt","LastModified":"2026-09-07T14:30:00Z"},
				{"Key":"sqlite/2026/09/06/new.db","LastModified":"2026-09-06T14:30:00Z"},
				{"Key":"other/wrong.db","LastModified":"2026-09-08T14:30:00Z"}
			]}"#,
			"archive",
			"sqlite/",
		)
		.unwrap()
		.as_str()
		.xpect_eq("sqlite/2026/09/06/new.db");
		newest_snapshot_key(r#"{"Contents":[]}"#, "archive", "sqlite/")
			.unwrap_err()
			.to_string()
			.xpect_contains("no SQLite .db snapshot");
	}

	#[beet_core::test]
	fn integrity_is_checked_before_the_live_database_is_named() {
		MailRestoreDrill::integrity_command()
			.as_str()
			.xpect_contains(MailRestoreDrill::REMOTE_PATH)
			.xpect_contains("PRAGMA integrity_check;")
			.xpect_contains("[ \"$result\" = ok ]")
			.xnot()
			.xpect_contains(StalwartBlock::DATABASE_PATH);
	}

	#[beet_core::test]
	fn replacement_removes_old_wal_and_shm() {
		MailRestoreDrill::replace_command()
			.as_str()
			.xpect_contains(&format!(
				"mv -f '{}' '{}'",
				MailRestoreDrill::REMOTE_PATH,
				StalwartBlock::DATABASE_PATH
			))
			.xpect_contains(&format!("{}-wal", StalwartBlock::DATABASE_PATH))
			.xpect_contains(&format!("{}-shm", StalwartBlock::DATABASE_PATH));
	}
}
