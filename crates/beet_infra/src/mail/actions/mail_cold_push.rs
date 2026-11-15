//! Running the box's cold-copy unit now rather than at its scheduled hour.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

impl MailColdPush {
	/// What a deploy runs on the box: the snapshot unit then the cold-copy
	/// unit, as a night where both timers fire runs them, started by hand.
	/// `systemctl start` on a oneshot blocks until it exits, so a failure is
	/// this command's exit status, and each script's own log line is read
	/// back so the deploy log says what was snapshotted and copied.
	pub fn command() -> String {
		format!(
			"sudo -n systemctl start {backup}.service && sudo -n systemctl \
			start {cold}.service && sudo -n grep -E 'mail database backed \
			up|cold copy:' {log} | tail -n 2",
			backup = StalwartBlock::BACKUP_UNIT,
			cold = StalwartBlock::COLD_UNIT,
			log = StalwartBlock::UNIT_LOG
		)
	}
}

/// Starts the snapshot and cold-copy units over ssh and waits for them.
/// `<MailColdPush/>` — run the box's nightly snapshot and cold copy now.
///
/// The timers are the schedule; this is the proof. A deploy that arms a timer
/// and walks away has proven that a timer exists, which is what the first
/// restore drill found the backup timer had been doing for a fortnight with
/// every check green. So a deploy runs the units itself, once, and the probe
/// after it reads the cold bucket back: the path is exercised end to end
/// before the first scheduled run, and again on every deploy after.
///
/// The snapshot runs first so a new box, whose archive holds nothing until
/// its first night, has one to copy, and so the copy the probe reads is
/// today's. The units are the ones the timers run, so nothing here can pass
/// that the nightly run would fail.
#[action]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
pub async fn MailColdPush(
	/// The private half of the key pair the box imported, as
	/// [`StalwartProvision`] takes it.
	#[field(default = StalwartProvision::SSH_KEY)]
	ssh_key: SmolStr,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let mail = MailStack::of(&cx.caller).await?;
	let cold = mail.cold_store()?;
	let connection = SshConnection {
		host: mail.public_ip().await?,
		user: StalwartProvision::SSH_USER.to_string(),
		port: 22,
		key_path: StalwartProvision::key_path(&ssh_key)?,
	};
	connection
		.wait_for_ready(Duration::from_secs(120), Duration::from_secs(5))
		.await?;
	info!(
		"running {} on {} against {}",
		StalwartBlock::COLD_UNIT,
		mail.mail_box.hostname(),
		cold.bucket_name(&mail.stack)
	);
	let output = connection.run_command(&MailColdPush::command()).await?;
	let line = String::from_utf8_lossy(&output.stdout);
	match line.trim().is_empty() {
		true => info!("cold copy ran, and logged nothing this run"),
		false => info!("{}", line.trim()),
	}
	Pass(cx.input).xok()
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The deploy starts the SAME units the timers do, the snapshot before
	/// the copy, so nothing can pass here that the nightly run would fail,
	/// and reads each script's own log line back rather than trusting the
	/// exit status alone.
	#[beet_core::test]
	fn the_push_runs_the_timers_units() {
		MailColdPush::command()
			.xpect_contains(
				"systemctl start stalwart-backup.service && sudo -n systemctl \
				start stalwart-cold-backup.service",
			)
			.xpect_contains("'mail database backed up|cold copy:'");
	}
}
