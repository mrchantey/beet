//! Persistent Stalwart data-volume snapshots before an infrastructure apply.
use crate::actions::aws_cli_ext;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;
use serde_json::json;

/// `<StalwartSnapshot/>` — snapshots the mail box's persistent data volume.
///
/// The step belongs immediately before `<TofuApply/>`. A first deploy has no
/// volume yet and is skipped; later deploys wait for a complete snapshot before
/// an apply can replace the instance or otherwise change its attachment. A
/// retry within one deploy reuses the snapshot that deploy already took, found
/// by its own tag ([`reusable`](Self::reusable)).
///
/// This is an INTERLOCK, not a backup schedule. It fires when a deploy is about
/// to do something dangerous, so its depth is how many recent deploys you can
/// roll back through, not how far back in time you can recover: the scheduled
/// backup is the box's own nightly `stalwart-backup.timer`, which produces a
/// verified, restore-anywhere copy that this snapshot cannot.
///
/// A snapshot every deploy is a lineage nothing else prunes, and EBS keeps one
/// until it is deleted, so the step prunes its own: after a snapshot completes,
/// every older one this block took of the same volume beyond
/// [`snapshot_retain`](Self::with_snapshot_retain) is deleted. Only snapshots
/// carrying this block's own tags are ever considered, so a manual snapshot
/// taken before something frightening is never swept up by the next deploy.
#[derive(Debug, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(StalwartSnapshotAction)]
pub struct StalwartSnapshot {
	/// How many of this block's own snapshots of the data volume survive a
	/// deploy, newest first: a rollback depth in DEPLOYS, not in days.
	#[get(copy)]
	snapshot_retain: usize,
}

impl Default for StalwartSnapshot {
	fn default() -> Self {
		Self {
			snapshot_retain: Self::SNAPSHOT_RETAIN,
		}
	}
}

impl StalwartSnapshot {
	/// The default rollback depth. Three deploys is enough to step back past a
	/// bad one and the one before it; time-based recovery is the nightly
	/// timer's job, not this step's.
	pub const SNAPSHOT_RETAIN: usize = 3;

	const DEPLOY_TAG: &'static str = "DeployId";
	const SOURCE_TAG: &'static str = "SourceVolumeName";

	/// The tag filters identifying this stack's data volume, so the step finds
	/// the declaration without reading tofu state.
	fn volume_filters(
		stack: &ResolvedStack,
		mail_box: &StalwartBlock,
	) -> [String; 3] {
		[
			format!("Name=tag:Name,Values={}", mail_box.data_volume_tag_name()),
			format!("Name=tag:Project,Values={}", stack.app_name()),
			format!("Name=tag:Stage,Values={}", stack.stage()),
		]
	}

	/// The tag filters identifying the snapshots this step has taken of that
	/// volume, ie the lineage it owns and may prune.
	fn snapshot_filters(
		stack: &ResolvedStack,
		mail_box: &StalwartBlock,
	) -> [String; 3] {
		[
			format!(
				"Name=tag:{},Values={}",
				Self::SOURCE_TAG,
				mail_box.data_volume_tag_name()
			),
			format!("Name=tag:Project,Values={}", stack.app_name()),
			format!("Name=tag:Stage,Values={}", stack.stage()),
		]
	}

	/// The tag filter narrowing that lineage to one deploy's snapshot.
	fn deploy_filter(deploy_id: &str) -> String {
		format!("Name=tag:{},Values={deploy_id}", Self::DEPLOY_TAG)
	}

	/// The snapshot an earlier attempt of THIS deploy already took, if any.
	///
	/// `CreateSnapshot` has no idempotency token — only the multi-volume
	/// `CreateSnapshots` does — so a retry is made idempotent by reading back
	/// the [`DEPLOY_TAG`](Self::DEPLOY_TAG) the step writes. The deploy id is
	/// stable across the whole sequence, so a re-run finds its own snapshot
	/// instead of taking a second. An `error` snapshot is not a backup and is
	/// passed over, so a retry after a failed capture takes a fresh one.
	fn reusable(body: &str) -> Result<Option<String>> {
		let listing: Value = serde_json::from_str(body)?;
		let mut snapshots = listing["Snapshots"]
			.as_array()
			.into_iter()
			.flatten()
			.filter_map(|snapshot| {
				let id = snapshot["SnapshotId"].as_str()?;
				let started = snapshot["StartTime"].as_str()?;
				matches!(
					snapshot["State"].as_str(),
					Some("completed" | "pending")
				)
				.then(|| (started.to_string(), id.to_string()))
			})
			.collect::<Vec<_>>();
		snapshots.sort_by(|left, right| right.0.cmp(&left.0));
		snapshots.into_iter().next().map(|(_, id)| id).xok()
	}

	fn volume_id(body: &str) -> Result<Option<String>> {
		let ids: Vec<String> = serde_json::from_str(body)?;
		match ids.as_slice() {
			[] => Ok(None),
			[id] => Ok(Some(id.clone())),
			_ => bevybail!(
				"the Stalwart data-volume tags matched several EBS volumes: {}",
				ids.join(", ")
			),
		}
	}

	fn tag_specification(
		stack: &ResolvedStack,
		mail_box: &StalwartBlock,
		deploy_id: &str,
	) -> String {
		let volume_name = mail_box.data_volume_tag_name();
		json!([{
			"ResourceType": "snapshot",
			"Tags": [
				{ "Key": "Name", "Value": format!("{volume_name}--snapshot") },
				{ "Key": "Project", "Value": stack.app_name() },
				{ "Key": "Stage", "Value": stack.stage() },
				{ "Key": Self::SOURCE_TAG, "Value": volume_name },
				{ "Key": Self::DEPLOY_TAG, "Value": deploy_id },
			]
		}])
		.to_string()
	}

	/// The snapshots to delete: this lineage, newest first, past `retain`.
	///
	/// Only `completed` snapshots count towards the retained window, so a run
	/// that fails between creating and completing one cannot push a good
	/// snapshot out of the lineage. `retain` of zero prunes nothing, since a
	/// step whose whole purpose is a backup should not be configurable into
	/// deleting the one it just took.
	fn prunable(body: &str, keep: &str, retain: usize) -> Result<Vec<String>> {
		if retain == 0 {
			return Vec::new().xok();
		}
		let listing: Value = serde_json::from_str(body)?;
		let mut snapshots = listing["Snapshots"]
			.as_array()
			.into_iter()
			.flatten()
			.filter_map(|snapshot| {
				let id = snapshot["SnapshotId"].as_str()?;
				let started = snapshot["StartTime"].as_str()?;
				(snapshot["State"].as_str() == Some("completed"))
					.then(|| (started.to_string(), id.to_string()))
			})
			.collect::<Vec<_>>();
		snapshots.sort_by(|left, right| right.0.cmp(&left.0));
		snapshots
			.into_iter()
			.map(|(_, id)| id)
			.filter(|id| id != keep)
			.skip(retain.saturating_sub(1))
			.collect::<Vec<_>>()
			.xok()
	}
}

/// Creates and waits for the current deploy's Stalwart data snapshot, then
/// prunes the lineage it belongs to.
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn StalwartSnapshotAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let retain = cx
		.caller
		.get::<StalwartSnapshot, _>(|snapshot| snapshot.snapshot_retain())
		.await
		.unwrap_or(StalwartSnapshot::SNAPSHOT_RETAIN);
	let mail = cx.caller.with_world(MailStack::resolve).await??;
	let region = mail.stack.region();
	let filters = StalwartSnapshot::volume_filters(&mail.stack, &mail.mail_box);
	let volume_id = aws_cli_ext::ec2(region, [
		"describe-volumes",
		"--filters",
		&filters[0],
		&filters[1],
		&filters[2],
		"--query",
		"Volumes[].VolumeId",
		"--output",
		"json",
	])
	.run_async_stdout()
	.await?
	.xmap(|body| StalwartSnapshot::volume_id(&body))?;
	let Some(volume_id) = volume_id else {
		info!(
			"no Stalwart data volume exists yet; skipping the first-deploy snapshot"
		);
		return Pass(cx.input).xok();
	};

	let deploy_id = mail.project.deployment().deploy_id().to_string();
	let description = format!("Stalwart data volume before deploy {deploy_id}");
	let tags = StalwartSnapshot::tag_specification(
		&mail.stack,
		&mail.mail_box,
		&deploy_id,
	);
	// a retry within one deploy reuses its own snapshot rather than taking a
	// second: the deploy id is stable across the whole sequence and rides the
	// snapshot as a tag, which is the only idempotency `CreateSnapshot` offers
	let lineage =
		StalwartSnapshot::snapshot_filters(&mail.stack, &mail.mail_box);
	let deploy_filter = StalwartSnapshot::deploy_filter(&deploy_id);
	let reusable = aws_cli_ext::ec2(region, [
		"describe-snapshots",
		"--owner-ids",
		"self",
		"--filters",
		&lineage[0],
		&lineage[1],
		&lineage[2],
		&deploy_filter,
		"--output",
		"json",
	])
	.run_async_stdout()
	.await?
	.xmap(|body| StalwartSnapshot::reusable(&body))?;

	let snapshot_id = match reusable {
		Some(snapshot_id) => {
			info!("reusing Stalwart snapshot {snapshot_id} from this deploy");
			snapshot_id
		}
		None => {
			info!("snapshotting Stalwart data volume {volume_id}");
			let snapshot_id = aws_cli_ext::ec2(region, [
				"create-snapshot",
				"--volume-id",
				&volume_id,
				"--description",
				&description,
				"--tag-specifications",
				&tags,
				"--query",
				"SnapshotId",
				"--output",
				"text",
			])
			.run_async_stdout()
			.await?
			.trim()
			.to_string();
			if snapshot_id.is_empty() || snapshot_id == "None" {
				bevybail!(
					"AWS created no snapshot id for Stalwart data volume {}",
					mail.mail_box.data_volume_tag_name()
				);
			}
			snapshot_id
		}
	};

	info!("waiting for Stalwart snapshot {snapshot_id}");
	aws_cli_ext::ec2(region, [
		"wait",
		"snapshot-completed",
		"--snapshot-ids",
		&snapshot_id,
	])
	.run_async()
	.await?;
	info!("Stalwart snapshot {snapshot_id} is complete");

	prune(&mail, &snapshot_id, retain).await?;
	Pass(cx.input).xok()
}

/// Deletes this block's own older snapshots of the data volume.
///
/// Pruning is deliberately after the wait: the lineage only shortens once the
/// snapshot replacing its oldest member is durable. A delete that fails is a
/// warning rather than a deploy failure, since the backup the step exists for
/// already succeeded and a leaked snapshot costs cents.
async fn prune(mail: &MailStack, keep: &str, retain: usize) -> Result {
	let region = mail.stack.region();
	let filters =
		StalwartSnapshot::snapshot_filters(&mail.stack, &mail.mail_box);
	let listing = aws_cli_ext::ec2(region, [
		"describe-snapshots",
		"--owner-ids",
		"self",
		"--filters",
		&filters[0],
		&filters[1],
		&filters[2],
		"--output",
		"json",
	])
	.run_async_stdout()
	.await?;
	for id in StalwartSnapshot::prunable(&listing, keep, retain)? {
		info!("deleting superseded Stalwart snapshot {id}");
		if let Err(err) =
			aws_cli_ext::ec2(region, ["delete-snapshot", "--snapshot-id", &id])
				.run_async()
				.await
		{
			warn!("could not delete superseded snapshot {id}: {err}");
		}
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	fn no_volume_is_first_deploy_state() {
		StalwartSnapshot::volume_id("[]").unwrap().xpect_none();
	}

	#[beet_core::test]
	fn exactly_one_tagged_volume_is_required() {
		StalwartSnapshot::volume_id(r#"["vol-123"]"#)
			.unwrap()
			.unwrap()
			.as_str()
			.xpect_eq("vol-123");
		StalwartSnapshot::volume_id(r#"["vol-123","vol-456"]"#)
			.unwrap_err()
			.to_string()
			.xpect_contains("matched several EBS volumes");
	}

	#[beet_core::test]
	fn snapshot_tags_bind_it_to_the_stack_volume_and_deploy() {
		let stack = Stack::new("mail-example")
			.with_stage("prod")
			.resolve(&PackageConfig::default());
		let mail_box = StalwartBlock::new("mail", "mail.example.com");
		let tags = StalwartSnapshot::tag_specification(
			&stack,
			&mail_box,
			"01990000-0000-7000-8000-000000000000",
		);
		tags.as_str()
			.xpect_contains(r#""Key":"Project","Value":"mail-example""#)
			.xpect_contains(r#""Key":"Stage","Value":"prod""#)
			.xpect_contains(r#""Key":"SourceVolumeName","Value":"mail--data""#)
			.xpect_contains(
				r#""Key":"DeployId","Value":"01990000-0000-7000-8000-000000000000""#,
			);
		// ..and the prune filters read back exactly the tag the snapshot carries
		StalwartSnapshot::snapshot_filters(&stack, &mail_box)[0]
			.as_str()
			.xpect_eq("Name=tag:SourceVolumeName,Values=mail--data");
	}

	/// One deploy's own snapshot is found by its tag rather than by an
	/// idempotency token, because `CreateSnapshot` accepts none: passing
	/// `--client-token` to it fails the whole deploy at the aws cli.
	#[beet_core::test]
	fn a_retry_reuses_this_deploys_snapshot() {
		StalwartSnapshot::deploy_filter("01990000-0000-7000-8000-000000000000")
			.as_str()
			.xpect_eq(
				"Name=tag:DeployId,Values=01990000-0000-7000-8000-000000000000",
			);
		// nothing yet, so the step creates one
		StalwartSnapshot::reusable(r#"{"Snapshots":[]}"#)
			.unwrap()
			.xpect_none();
		// a capture still running is this deploy's, and is waited on rather
		// than duplicated
		StalwartSnapshot::reusable(
			r#"{"Snapshots":[{"SnapshotId":"snap-pending","StartTime":"2026-09-03T00:00:00+00:00","State":"pending"}]}"#,
		)
		.unwrap()
		.unwrap()
		.as_str()
		.xpect_eq("snap-pending");
		StalwartSnapshot::reusable(LISTING)
			.unwrap()
			.unwrap()
			.as_str()
			.xpect_eq("snap-new");
	}

	/// A failed capture is not a backup, so the retry takes a fresh one rather
	/// than waiting forever on a snapshot that will never complete.
	#[beet_core::test]
	fn an_errored_snapshot_is_never_reused() {
		StalwartSnapshot::reusable(
			r#"{"Snapshots":[{"SnapshotId":"snap-bad","StartTime":"2026-09-05T00:00:00+00:00","State":"error"}]}"#,
		)
		.unwrap()
		.xpect_none();
	}

	const LISTING: &str = r#"{"Snapshots":[
		{"SnapshotId":"snap-old","StartTime":"2026-09-01T00:00:00+00:00","State":"completed"},
		{"SnapshotId":"snap-new","StartTime":"2026-09-04T00:00:00+00:00","State":"completed"},
		{"SnapshotId":"snap-mid","StartTime":"2026-09-02T00:00:00+00:00","State":"completed"},
		{"SnapshotId":"snap-pending","StartTime":"2026-09-03T00:00:00+00:00","State":"pending"}
	]}"#;

	/// The lineage keeps `retain` snapshots counting the one just taken, so a
	/// deploy leaves exactly that many behind however often it runs.
	#[beet_core::test]
	fn prunes_the_oldest_beyond_the_retained_window() {
		StalwartSnapshot::prunable(LISTING, "snap-new", 2)
			.unwrap()
			.xpect_eq(vec!["snap-old".to_string()]);
		StalwartSnapshot::prunable(LISTING, "snap-new", 1)
			.unwrap()
			.xpect_eq(vec!["snap-mid".to_string(), "snap-old".to_string()]);
		StalwartSnapshot::prunable(LISTING, "snap-new", 7)
			.unwrap()
			.is_empty()
			.xpect_true();
	}

	/// An incomplete snapshot is not a backup, so it neither occupies a slot in
	/// the window nor is deleted out from under the run that is creating it.
	#[beet_core::test]
	fn only_completed_snapshots_are_counted_and_the_new_one_is_never_deleted() {
		let prunable = StalwartSnapshot::prunable(LISTING, "snap-new", 1)
			.unwrap()
			.join(",");
		prunable
			.as_str()
			.xnot()
			.xpect_contains("snap-pending")
			.xnot()
			.xpect_contains("snap-new");
		// ..and a retain of zero prunes nothing rather than everything
		StalwartSnapshot::prunable(LISTING, "snap-new", 0)
			.unwrap()
			.is_empty()
			.xpect_true();
	}
}
