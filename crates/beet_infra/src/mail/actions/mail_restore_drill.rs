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
	let mail = cx
		.caller
		.with_state::<MailQuery, _>(|entity, query| query.resolve(entity))
		.await??;
	let stage = mail.stack.stage().clone();
	if &stage == drill.source_stage() {
		bevybail!(
			"this drill would restore {stage}'s own backup over {stage}'s live \
			database. Deploy the stack to a throwaway stage and run it there: \
			`--stage=drill`"
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
		.scp_to(local.as_ref(), MailRestoreDrill::REMOTE_PATH)
		.await?;
	fs_ext::remove(&local).ok();

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
	let output = connection.run_command(&restore_command(&mail)).await?;
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

	assert_restored(&mail, &source, drill.mailbox(), *drill.timeout()).await?;
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
async fn assert_restored(
	mail: &MailStack,
	source: &ResolvedStack,
	localpart: &str,
	timeout: Duration,
) -> Result {
	let domain = mail.domain_holding(localpart)?;
	let address = format!("{localpart}@{}", domain.domain());
	let secret = SecretRef::new(format!(
		"{}-account-{localpart}-at-{}",
		mail.mail_box.label(),
		domain.slug()
	))
	.name(source);
	let password = ssm_ext::get(&mail.stack.region(), &secret)
		.await?
		.ok_or_else(|| {
			bevyhow!(
				"no credential at {secret}: the drill authenticates as one of 				the SOURCE stage's accounts, so its parameters must still exist"
			)
		})?;

	// the box by its own hostname, since the mail domain's records point at
	// whichever box the owning stage stood up
	let origin = format!("https://{}", mail.mail_box.hostname());
	let poll = Duration::from_secs(5);
	let attempts = (timeout.as_secs() / poll.as_secs()).max(1);
	let mut last = None;
	for _ in 0..attempts {
		match JmapClient::connect(&origin, &address, &password).await {
			Ok(client) => {
				let account = client.mail_account()?.to_string();
				let mailboxes = client
					.call_mail("Mailbox/get", json!({ "accountId": account }))
					.await?["list"]
					.as_array()
					.map(Vec::len)
					.unwrap_or_default();
				if mailboxes == 0 {
					bevybail!(
						"{address} authenticated against the restored store but 						holds no mailboxes, so the schema came back and the 						data did not"
					);
				}
				info!("{address} signed in and holds {mailboxes} mailbox(es)");
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
/// no secret rides this command line. `--clean --if-exists` because a drill
/// stage has already been provisioned with its own empty mailboxes, and
/// `--no-owner --no-acl` because the dump's roles are the source stage's.
fn restore_command(mail: &MailStack) -> String {
	let secret = mail.mail_box.database().secret_name(&mail.stack);
	format!(
		"sudo -n -u stalwart env \
		PGPASSWORD=\"$(aws ssm get-parameter --region '{region}' --name '{secret}' --with-decryption --query Parameter.Value --output text)\" \
		PGSSLMODE=verify-full \
		pg_restore --host '{host}' --port {port} --username '{user}' \
		--dbname '{database}' --clean --if-exists --no-owner --no-acl \
		'{path}'; sudo -n rm -f '{path}'",
		region = mail.stack.region(),
		host = mail.mail_box.database().host(&mail.stack),
		port = RdsPostgresBlock::PORT,
		user = mail.mail_box.db_user(),
		database = mail.mail_box.db_name(),
		path = MailRestoreDrill::REMOTE_PATH,
	)
}
