//! Persistent Stalwart data-volume snapshots before an infrastructure apply.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// An EC2 `Filter`: a name and the one value it matches.
type Filter = (String, String);

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
	) -> [Filter; 3] {
		[
			Self::tag_filter("Name", mail_box.data_volume_tag_name()),
			Self::tag_filter("Project", stack.app_name()),
			Self::tag_filter("Stage", stack.stage()),
		]
	}

	/// The tag filters identifying the snapshots this step has taken of that
	/// volume, ie the lineage it owns and may prune.
	fn snapshot_filters(
		stack: &ResolvedStack,
		mail_box: &StalwartBlock,
	) -> [Filter; 3] {
		[
			Self::tag_filter(Self::SOURCE_TAG, mail_box.data_volume_tag_name()),
			Self::tag_filter("Project", stack.app_name()),
			Self::tag_filter("Stage", stack.stage()),
		]
	}

	/// The tag filter narrowing that lineage to one deploy's snapshot.
	fn deploy_filter(deploy_id: &str) -> Filter {
		Self::tag_filter(Self::DEPLOY_TAG, deploy_id)
	}

	fn tag_filter(key: &str, value: impl ToString) -> Filter {
		(format!("tag:{key}"), value.to_string())
	}

	/// The snapshot an earlier attempt of THIS deploy already took, if any.
	///
	/// `CreateSnapshot` has no idempotency token — only the multi-volume
	/// `CreateSnapshots` does — so a retry is made idempotent by reading back
	/// the [`DEPLOY_TAG`](Self::DEPLOY_TAG) the step writes. The deploy id is
	/// stable across the whole sequence, so a re-run finds its own snapshot
	/// instead of taking a second. An `error` snapshot is not a backup and is
	/// passed over, so a retry after a failed capture takes a fresh one.
	fn reusable(snapshots: &[Snapshot]) -> Option<String> {
		snapshots
			.iter()
			.filter(|snapshot| {
				matches!(snapshot.state.as_str(), "completed" | "pending")
			})
			.max_by(|left, right| left.start_time.cmp(&right.start_time))
			.map(|snapshot| snapshot.id.clone())
	}

	fn volume_id(ids: &[String]) -> Result<Option<String>> {
		match ids {
			[] => Ok(None),
			[id] => Ok(Some(id.clone())),
			_ => bevybail!(
				"the Stalwart data-volume tags matched several EBS volumes: {}",
				ids.join(", ")
			),
		}
	}

	/// The tags a snapshot carries: its name, the stack's, and the volume and
	/// deploy it was taken of.
	fn tags(
		stack: &ResolvedStack,
		mail_box: &StalwartBlock,
		deploy_id: &str,
	) -> Vec<(&'static str, String)> {
		let volume_name = mail_box.data_volume_tag_name();
		vec![
			("Name", format!("{volume_name}--snapshot")),
			("Project", stack.app_name().to_string()),
			("Stage", stack.stage().to_string()),
			(Self::SOURCE_TAG, volume_name.to_string()),
			(Self::DEPLOY_TAG, deploy_id.to_string()),
		]
	}

	/// The snapshots to delete: this lineage, newest first, past `retain`.
	///
	/// Only `completed` snapshots count towards the retained window, so a run
	/// that fails between creating and completing one cannot push a good
	/// snapshot out of the lineage. `retain` of zero prunes nothing, since a
	/// step whose whole purpose is a backup should not be configurable into
	/// deleting the one it just took.
	fn prunable(
		snapshots: &[Snapshot],
		keep: &str,
		retain: usize,
	) -> Vec<String> {
		if retain == 0 {
			return Vec::new();
		}
		let mut completed = snapshots
			.iter()
			.filter(|snapshot| snapshot.state == "completed")
			.collect::<Vec<_>>();
		completed.sort_by(|left, right| right.start_time.cmp(&left.start_time));
		completed
			.into_iter()
			.map(|snapshot| snapshot.id.clone())
			.filter(|id| id != keep)
			.skip(retain.saturating_sub(1))
			.collect()
	}
}

/// One snapshot as `DescribeSnapshots` reports it, as much as the step reads.
#[derive(Debug, Clone, PartialEq)]
struct Snapshot {
	id: String,
	/// ISO 8601, so a lexical comparison is a chronological one.
	start_time: String,
	/// `pending`, `completed`, `error`, ..
	state: String,
}

impl Snapshot {
	/// The snapshot an `item` of a `snapshotSet` describes.
	fn parse(item: &BsxElement) -> Option<Self> {
		Self {
			id: item.text_at("snapshotId")?.to_string(),
			start_time: item.text_at("startTime")?.to_string(),
			state: item.text_at("status")?.to_string(),
		}
		.xsome()
	}
}

/// The EBS calls the step makes, over one EC2 client in the stack's region,
/// signed as the deployer.
struct Ebs {
	client: AwsClient,
	/// The gap between [`wait_completed`](Self::wait_completed) attempts.
	poll: Duration,
}

impl Ebs {
	/// How many times [`wait_completed`](Self::wait_completed) asks, every
	/// [`POLL`](Self::POLL): ten minutes, as long as the `aws` cli's own
	/// `snapshot-completed` waiter.
	const ATTEMPTS: usize = 40;
	const POLL: Duration = Duration::from_secs(15);

	fn new(region: &str) -> Self {
		Self {
			client: AwsClient::new(AwsService::EC2, region),
			poll: Self::POLL,
		}
	}

	/// `filters` as ec2Query params, `Filter.N.Name` and `Filter.N.Value.1`.
	fn filter_params(filters: &[Filter]) -> Vec<(String, String)> {
		filters
			.iter()
			.enumerate()
			.flat_map(|(index, (name, value))| {
				let index = index + 1;
				[
					(format!("Filter.{index}.Name"), name.clone()),
					(format!("Filter.{index}.Value.1"), value.clone()),
				]
			})
			.collect()
	}

	/// Every page of a describe `action` with `params`, followed by
	/// `nextToken`.
	async fn pages(
		&self,
		action: &str,
		params: Vec<(String, String)>,
	) -> Result<Vec<BsxElement>> {
		let mut pages = Vec::new();
		let mut next_token: Option<String> = None;
		loop {
			let mut page_params = params.clone();
			if let Some(token) = next_token.take() {
				page_params.push(("NextToken".into(), token));
			}
			let page = self.client.call_query(action, page_params).await?;
			next_token = page
				.text_at("nextToken")
				.filter(|token| !token.is_empty())
				.map(ToString::to_string);
			pages.push(page);
			if next_token.is_none() {
				break;
			}
		}
		pages.xok()
	}

	/// `DescribeVolumes`: the id of every volume matching `filters`.
	async fn volume_ids(&self, filters: &[Filter]) -> Result<Vec<String>> {
		self.pages("DescribeVolumes", Self::filter_params(filters))
			.await?
			.iter()
			.flat_map(|page| page.at("volumeSet").into_iter())
			.flat_map(|set| set.children_named("item"))
			.filter_map(|item| {
				item.text_at("volumeId").map(ToString::to_string)
			})
			.collect::<Vec<_>>()
			.xok()
	}

	/// `DescribeSnapshots` of the account's own snapshots matching `filters`.
	async fn snapshots(&self, filters: &[Filter]) -> Result<Vec<Snapshot>> {
		[("Owner.1".to_string(), "self".to_string())]
			.into_iter()
			.chain(Self::filter_params(filters))
			.collect::<Vec<_>>()
			.xmap(|params| self.pages("DescribeSnapshots", params))
			.await?
			.xmap(|pages| Self::parse_snapshots(&pages))
			.xok()
	}

	fn parse_snapshots(pages: &[BsxElement]) -> Vec<Snapshot> {
		pages
			.iter()
			.flat_map(|page| page.at("snapshotSet").into_iter())
			.flat_map(|set| set.children_named("item"))
			.filter_map(Snapshot::parse)
			.collect()
	}

	/// `CreateSnapshot` of `volume_id`, tagged with `tags`, answering the
	/// new snapshot's id.
	async fn create(
		&self,
		volume_id: &str,
		description: &str,
		tags: &[(&str, String)],
	) -> Result<String> {
		let params = [
			("VolumeId".to_string(), volume_id.to_string()),
			("Description".to_string(), description.to_string()),
			(
				"TagSpecification.1.ResourceType".to_string(),
				"snapshot".to_string(),
			),
		]
		.into_iter()
		.chain(tags.iter().enumerate().flat_map(|(index, (key, value))| {
			let index = index + 1;
			[
				(
					format!("TagSpecification.1.Tag.{index}.Key"),
					key.to_string(),
				),
				(
					format!("TagSpecification.1.Tag.{index}.Value"),
					value.clone(),
				),
			]
		}))
		.collect::<Vec<_>>();
		let response = self.client.call_query("CreateSnapshot", params).await?;
		match response.text_at("snapshotId") {
			Some(id) if !id.is_empty() => id.to_string().xok(),
			_ => bevybail!("AWS created no snapshot id for volume {volume_id}"),
		}
	}

	/// Poll `snapshot_id` until it is `completed`, failing on `error` or
	/// after [`ATTEMPTS`](Self::ATTEMPTS).
	async fn wait_completed(&self, snapshot_id: &str) -> Result {
		for attempt in 1..=Self::ATTEMPTS {
			let state = self
				.pages("DescribeSnapshots", vec![(
					"SnapshotId.1".into(),
					snapshot_id.into(),
				)])
				.await?
				.xmap(|pages| Self::parse_snapshots(&pages))
				.into_iter()
				.find(|snapshot| snapshot.id == snapshot_id)
				.map(|snapshot| snapshot.state);
			match state.as_deref() {
				Some("completed") => return Ok(()),
				Some("error") => {
					bevybail!(
						"snapshot {snapshot_id} failed: its state is `error`"
					)
				}
				state => debug!(
					"snapshot {snapshot_id} is {} after {attempt} attempt(s)",
					state.unwrap_or("not yet listed")
				),
			}
			time_ext::sleep(self.poll).await;
		}
		bevybail!(
			"snapshot {snapshot_id} did not complete within {} attempts {:?} apart",
			Self::ATTEMPTS,
			self.poll
		)
	}

	/// `DeleteSnapshot`.
	async fn delete(&self, snapshot_id: &str) -> Result {
		self.client
			.call_query("DeleteSnapshot", [("SnapshotId", snapshot_id)])
			.await?;
		Ok(())
	}
}

/// Creates and waits for the current deploy's Stalwart data snapshot, then
/// prunes the lineage it belongs to.
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
/// What a *scheduled* volume snapshot would add over that timer is protection
/// against filesystem-level corruption the online `.backup` would faithfully
/// copy. It is deliberately not built: a verb that runs when a human deploys
/// can never be a schedule, so if it is ever wanted it belongs in an AWS Data
/// Lifecycle Manager policy with its own binding and block, declaring its
/// schedule and retention, rather than in more work here.
///
/// A snapshot every deploy is a lineage nothing else prunes, and EBS keeps one
/// until it is deleted, so the step prunes its own: after a snapshot completes,
/// every older one this block took of the same volume beyond
/// [`snapshot_retain`](StalwartSnapshot::snapshot_retain) is deleted. Only snapshots
/// carrying this block's own tags are ever considered, so a manual snapshot
/// taken before something frightening is never swept up by the next deploy.
#[action]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
pub async fn StalwartSnapshot(
	/// How many of this block's own snapshots of the data volume survive a
	/// deploy, newest first: a rollback depth in DEPLOYS, not in days.
	#[field(default = Self::SNAPSHOT_RETAIN)]
	snapshot_retain: usize,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let mail = MailStack::of(&cx.caller).await?;
	let ebs = Ebs::new(mail.stack.aws_region()?);
	let filters = StalwartSnapshot::volume_filters(&mail.stack, &mail.mail_box);
	let volume_id =
		StalwartSnapshot::volume_id(&ebs.volume_ids(&filters).await?)?;
	let Some(volume_id) = volume_id else {
		info!(
			"no Stalwart data volume exists yet; skipping the first-deploy snapshot"
		);
		return Pass(cx.input).xok();
	};

	let deploy_id = mail.project.deployment().deploy_id().to_string();
	// a retry within one deploy reuses its own snapshot rather than taking a
	// second: the deploy id is stable across the whole sequence and rides the
	// snapshot as a tag, which is the only idempotency `CreateSnapshot` offers
	let [name, project, stage] =
		StalwartSnapshot::snapshot_filters(&mail.stack, &mail.mail_box);
	let reusable = ebs
		.snapshots(&[
			name,
			project,
			stage,
			StalwartSnapshot::deploy_filter(&deploy_id),
		])
		.await?
		.xmap(|snapshots| StalwartSnapshot::reusable(&snapshots));

	let snapshot_id = match reusable {
		Some(snapshot_id) => {
			info!("reusing Stalwart snapshot {snapshot_id} from this deploy");
			snapshot_id
		}
		None => {
			info!("snapshotting Stalwart data volume {volume_id}");
			ebs.create(
				&volume_id,
				&format!("Stalwart data volume before deploy {deploy_id}"),
				&StalwartSnapshot::tags(
					&mail.stack,
					&mail.mail_box,
					&deploy_id,
				),
			)
			.await?
		}
	};

	info!("waiting for Stalwart snapshot {snapshot_id}");
	ebs.wait_completed(&snapshot_id).await?;
	info!("Stalwart snapshot {snapshot_id} is complete");

	prune(&ebs, &mail, &snapshot_id, snapshot_retain).await?;
	Pass(cx.input).xok()
}

/// Deletes this block's own older snapshots of the data volume.
///
/// Pruning is deliberately after the wait: the lineage only shortens once the
/// snapshot replacing its oldest member is durable. A delete that fails is a
/// warning rather than a deploy failure, since the backup the step exists for
/// already succeeded and a leaked snapshot costs cents.
async fn prune(
	ebs: &Ebs,
	mail: &MailStack,
	keep: &str,
	retain: usize,
) -> Result {
	let filters =
		StalwartSnapshot::snapshot_filters(&mail.stack, &mail.mail_box);
	let snapshots = ebs.snapshots(&filters).await?;
	for id in StalwartSnapshot::prunable(&snapshots, keep, retain) {
		info!("deleting superseded Stalwart snapshot {id}");
		if let Err(err) = ebs.delete(&id).await {
			warn!("could not delete superseded snapshot {id}: {err}");
		}
	}
	Ok(())
}

/// Deletes every snapshot this block's lineage holds for the stage, once the
/// volume they were taken of is gone.
/// `<StalwartSnapshotPrune/>` — the teardown half of [`StalwartSnapshot`], for
/// a stage whose data is disposable.
///
/// A snapshot every deploy is a lineage nothing else prunes, and a destroyed
/// stage's lineage outlives its volume: two `Stage=drill` snapshots after every
/// drill that deployed twice, deleted by hand each time. This is the sweep,
/// and it is declared in a teardown group rather than run by [`StackTeardown`]
/// because it must be OPT-IN: a production stack's last snapshots are the last
/// copies of its mail, and a destroy that took them would be the incident.
/// It refuses on a protected volume for that reason, so a tag copied into the
/// wrong entry fails rather than prunes.
///
/// After `<TofuDestroy/>` in convergence order, since the snapshot is taken
/// before the apply: the destroy takes the volume, then this takes its
/// history. Every snapshot carrying the lineage tags goes, whatever its state,
/// and a snapshot taken by hand (which carries none of them) never does.
#[action]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
pub async fn StalwartSnapshotPrune(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let mail = MailStack::of(&cx.caller).await?;
	if mail.mail_box.data_volume_protected(&mail.stack) {
		bevybail!(
			"stage '{}' protects its data volume, so its snapshots are the \
			last copies of its data and are never pruned by a destroy",
			mail.stack.stage()
		);
	}
	let ebs = Ebs::new(mail.stack.aws_region()?);
	let filters =
		StalwartSnapshot::snapshot_filters(&mail.stack, &mail.mail_box);
	let ids = ebs
		.snapshots(&filters)
		.await?
		.into_iter()
		.map(|snapshot| snapshot.id)
		.collect::<Vec<_>>();
	if ids.is_empty() {
		info!("no snapshots in the {} lineage", mail.stack.stage());
		return Pass(cx.input).xok();
	}
	for id in &ids {
		ebs.delete(id).await?;
		info!("deleted snapshot {id}");
	}
	info!(
		"pruned the {} lineage: {} snapshot(s)",
		mail.stack.stage(),
		ids.len()
	);
	Pass(cx.input).xok()
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::Arc;
	use std::sync::Mutex;

	/// One call's form params.
	type Params = MultiMap<SmolStr, SmolStr>;

	/// Every call an [`Ebs`] made.
	type Calls = Arc<Mutex<Vec<Params>>>;

	/// An [`Ebs`] in `ap-southeast-2` whose EC2 answers `answer`, given each
	/// call's form params, every call recorded; it polls without waiting.
	fn recording(
		answer: impl 'static + Send + Sync + Fn(&Params) -> String,
	) -> (Ebs, Calls) {
		let calls = Calls::default();
		let recorded = calls.clone();
		let answer = Arc::new(answer);
		let client = AwsClient::new(AwsService::EC2, "ap-southeast-2")
			.with_credentials(AwsCredentials::new("AKID", "secret"))
			.with_transport(move |request| {
				let (recorded, answer) = (recorded.clone(), answer.clone());
				Box::pin(async move {
					request
						.headers
						.first_raw("authorization")
						.unwrap()
						.xpect_contains("/ap-southeast-2/ec2/aws4_request");
					let body = request.body.into_bytes().await?;
					let params =
						Url::parse_query_string(str::from_utf8(&body)?);
					let response = Response::ok().with_body(answer(&params));
					recorded.lock().unwrap().push(params);
					response.xok()
				})
			});
		let ebs = Ebs {
			client,
			poll: Duration::ZERO,
		};
		(ebs, calls)
	}

	/// The value of `key` in `params`.
	fn param<'a>(params: &'a Params, key: &str) -> Option<&'a str> {
		params.get(key).map(SmolStr::as_str)
	}

	/// A `DescribeSnapshots` page of `snapshots` (id, start time, status),
	/// continued by `next`.
	fn snapshot_page(
		snapshots: &[(&str, &str, &str)],
		next: Option<&str>,
	) -> String {
		let items = snapshots
			.iter()
			.map(|(id, started, status)| {
				format!(
					"<item><snapshotId>{id}</snapshotId><volumeId>vol-1</volumeId>\
					<status>{status}</status><startTime>{started}</startTime>\
					<progress>100%</progress><ownerId>111122223333</ownerId>\
					<tagSet><item><key>Stage</key><value>prod</value></item></tagSet></item>"
				)
			})
			.collect::<String>();
		let next = next
			.map(|next| format!("<nextToken>{next}</nextToken>"))
			.unwrap_or_default();
		format!(
			"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
			<DescribeSnapshotsResponse xmlns=\"http://ec2.amazonaws.com/doc/2016-11-15/\">\
			<requestId>req-1</requestId><snapshotSet>{items}</snapshotSet>{next}\
			</DescribeSnapshotsResponse>"
		)
	}

	fn snapshot(id: &str, start_time: &str, state: &str) -> Snapshot {
		Snapshot {
			id: id.into(),
			start_time: start_time.into(),
			state: state.into(),
		}
	}

	fn listing() -> Vec<Snapshot> {
		vec![
			snapshot("snap-old", "2026-09-01T00:00:00.000Z", "completed"),
			snapshot("snap-new", "2026-09-04T00:00:00.000Z", "completed"),
			snapshot("snap-mid", "2026-09-02T00:00:00.000Z", "completed"),
			snapshot("snap-pending", "2026-09-03T00:00:00.000Z", "pending"),
		]
	}

	#[beet_core::test]
	fn no_volume_is_first_deploy_state() {
		StalwartSnapshot::volume_id(&[]).unwrap().xpect_none();
	}

	/// The volume is found by its tags, and exactly one may match.
	#[beet_core::test]
	async fn exactly_one_tagged_volume_is_required() {
		let (ebs, calls) = recording(|_| {
			"<DescribeVolumesResponse><requestId>req-1</requestId><volumeSet>\
			<item><volumeId>vol-123</volumeId><size>8</size><status>in-use</status></item>\
			</volumeSet></DescribeVolumesResponse>"
				.into()
		});
		let filters = [
			StalwartSnapshot::tag_filter("Name", "mail--data"),
			StalwartSnapshot::tag_filter("Stage", "prod"),
		];
		let ids = ebs.volume_ids(&filters).await.unwrap();
		StalwartSnapshot::volume_id(&ids)
			.unwrap()
			.unwrap()
			.as_str()
			.xpect_eq("vol-123");
		let calls = calls.lock().unwrap().clone();
		param(&calls[0], "Action").xpect_eq(Some("DescribeVolumes"));
		param(&calls[0], "Version").xpect_eq(Some("2016-11-15"));
		param(&calls[0], "Filter.1.Name").xpect_eq(Some("tag:Name"));
		param(&calls[0], "Filter.1.Value.1").xpect_eq(Some("mail--data"));
		param(&calls[0], "Filter.2.Name").xpect_eq(Some("tag:Stage"));
		param(&calls[0], "Filter.2.Value.1").xpect_eq(Some("prod"));
		StalwartSnapshot::volume_id(&["vol-123".into(), "vol-456".into()])
			.unwrap_err()
			.to_string()
			.xpect_contains("matched several EBS volumes");
	}

	/// The account's own snapshots, every page of them, the token carried
	/// from one to the next.
	#[beet_core::test]
	async fn snapshots_are_listed_across_pages() {
		let (ebs, calls) =
			recording(|params| match param(params, "NextToken") {
				None => snapshot_page(
					&[("snap-1", "2026-09-01T00:00:00.000Z", "completed")],
					Some("token-1"),
				),
				Some(_) => snapshot_page(
					&[("snap-2", "2026-09-02T00:00:00.000Z", "pending")],
					None,
				),
			});
		ebs.snapshots(&[StalwartSnapshot::deploy_filter("deploy-1")])
			.await
			.unwrap()
			.xpect_eq(vec![
				snapshot("snap-1", "2026-09-01T00:00:00.000Z", "completed"),
				snapshot("snap-2", "2026-09-02T00:00:00.000Z", "pending"),
			]);
		let calls = calls.lock().unwrap().clone();
		calls.len().xpect_eq(2);
		param(&calls[0], "Action").xpect_eq(Some("DescribeSnapshots"));
		param(&calls[0], "Owner.1").xpect_eq(Some("self"));
		param(&calls[0], "Filter.1.Name").xpect_eq(Some("tag:DeployId"));
		param(&calls[0], "Filter.1.Value.1").xpect_eq(Some("deploy-1"));
		param(&calls[0], "NextToken").xpect_none();
		param(&calls[1], "NextToken").xpect_eq(Some("token-1"));
	}

	/// The snapshot is created of the volume with its description and
	/// every tag in one snapshot tag specification, answering its id.
	#[beet_core::test]
	async fn creates_a_tagged_snapshot() {
		let stack = Stack::new("mail-example")
			.with_stage("prod")
			.resolve(&PackageConfig::default());
		let mail_box = StalwartBlock::new("mail", "mail.example.com");
		let tags = StalwartSnapshot::tags(
			&stack,
			&mail_box,
			"01990000-0000-7000-8000-000000000000",
		);
		let (ebs, calls) = recording(|_| {
			"<CreateSnapshotResponse><requestId>req-1</requestId>\
			<snapshotId>snap-new</snapshotId><volumeId>vol-1</volumeId>\
			<status>pending</status><startTime>2026-09-04T00:00:00.000Z</startTime>\
			</CreateSnapshotResponse>"
				.into()
		});
		ebs.create("vol-1", "before deploy", &tags)
			.await
			.unwrap()
			.as_str()
			.xpect_eq("snap-new");
		let calls = calls.lock().unwrap().clone();
		let params = &calls[0];
		param(params, "Action").xpect_eq(Some("CreateSnapshot"));
		param(params, "VolumeId").xpect_eq(Some("vol-1"));
		param(params, "Description").xpect_eq(Some("before deploy"));
		param(params, "TagSpecification.1.ResourceType")
			.xpect_eq(Some("snapshot"));
		let tag = |index: usize| {
			(
				param(params, &format!("TagSpecification.1.Tag.{index}.Key")),
				param(params, &format!("TagSpecification.1.Tag.{index}.Value")),
			)
		};
		tag(1).xpect_eq((Some("Name"), Some("mail--data--snapshot")));
		tag(2).xpect_eq((Some("Project"), Some("mail-example")));
		tag(3).xpect_eq((Some("Stage"), Some("prod")));
		tag(4).xpect_eq((Some("SourceVolumeName"), Some("mail--data")));
		tag(5).xpect_eq((
			Some("DeployId"),
			Some("01990000-0000-7000-8000-000000000000"),
		));
		// ..and the prune filters read back exactly the tag the snapshot carries
		StalwartSnapshot::snapshot_filters(&stack, &mail_box)[0].xpect_eq((
			"tag:SourceVolumeName".to_string(),
			"mail--data".to_string(),
		));
	}

	/// The wait asks after the one snapshot until it reads `completed`.
	#[beet_core::test]
	async fn waits_until_the_snapshot_completes() {
		let answers = Arc::new(Mutex::new(
			vec!["pending", "pending", "completed"].into_iter(),
		));
		let (ebs, calls) = recording(move |_| {
			let status = answers.lock().unwrap().next().unwrap();
			snapshot_page(
				&[("snap-1", "2026-09-01T00:00:00.000Z", status)],
				None,
			)
		});
		ebs.wait_completed("snap-1").await.unwrap();
		let calls = calls.lock().unwrap().clone();
		calls.len().xpect_eq(3);
		param(&calls[0], "SnapshotId.1").xpect_eq(Some("snap-1"));
		param(&calls[0], "Owner.1").xpect_none();
	}

	/// A snapshot in `error` fails the wait at once, and one that never
	/// completes fails it after the waiter's bound.
	#[beet_core::test]
	async fn an_errored_or_stuck_snapshot_fails_the_wait() {
		let (ebs, calls) = recording(|_| {
			snapshot_page(
				&[("snap-1", "2026-09-01T00:00:00.000Z", "error")],
				None,
			)
		});
		ebs.wait_completed("snap-1")
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("`error`");
		calls.lock().unwrap().len().xpect_eq(1);
		let (ebs, calls) = recording(|_| {
			snapshot_page(
				&[("snap-1", "2026-09-01T00:00:00.000Z", "pending")],
				None,
			)
		});
		ebs.wait_completed("snap-1").await.unwrap_err();
		calls.lock().unwrap().len().xpect_eq(Ebs::ATTEMPTS);
	}

	/// The delete names the one snapshot.
	#[beet_core::test]
	async fn deletes_a_snapshot() {
		let (ebs, calls) = recording(|_| {
			"<DeleteSnapshotResponse><requestId>req-1</requestId>\
			<return>true</return></DeleteSnapshotResponse>"
				.into()
		});
		ebs.delete("snap-old").await.unwrap();
		let calls = calls.lock().unwrap().clone();
		param(&calls[0], "Action").xpect_eq(Some("DeleteSnapshot"));
		param(&calls[0], "SnapshotId").xpect_eq(Some("snap-old"));
	}

	/// One deploy's own snapshot is found by its tag rather than by an
	/// idempotency token, because `CreateSnapshot` accepts none.
	#[beet_core::test]
	fn a_retry_reuses_this_deploys_snapshot() {
		StalwartSnapshot::deploy_filter("01990000-0000-7000-8000-000000000000")
			.xpect_eq((
				"tag:DeployId".to_string(),
				"01990000-0000-7000-8000-000000000000".to_string(),
			));
		// nothing yet, so the step creates one
		StalwartSnapshot::reusable(&[]).xpect_none();
		// a capture still running is this deploy's, and is waited on rather
		// than duplicated
		StalwartSnapshot::reusable(&[snapshot(
			"snap-pending",
			"2026-09-03T00:00:00.000Z",
			"pending",
		)])
		.unwrap()
		.as_str()
		.xpect_eq("snap-pending");
		StalwartSnapshot::reusable(&listing())
			.unwrap()
			.as_str()
			.xpect_eq("snap-new");
	}

	/// A failed capture is not a backup, so the retry takes a fresh one rather
	/// than waiting forever on a snapshot that will never complete.
	#[beet_core::test]
	fn an_errored_snapshot_is_never_reused() {
		StalwartSnapshot::reusable(&[snapshot(
			"snap-bad",
			"2026-09-05T00:00:00.000Z",
			"error",
		)])
		.xpect_none();
	}

	/// The lineage keeps `retain` snapshots counting the one just taken, so a
	/// deploy leaves exactly that many behind however often it runs.
	#[beet_core::test]
	fn prunes_the_oldest_beyond_the_retained_window() {
		let listing = listing();
		StalwartSnapshot::prunable(&listing, "snap-new", 2)
			.xpect_eq(vec!["snap-old".to_string()]);
		StalwartSnapshot::prunable(&listing, "snap-new", 1)
			.xpect_eq(vec!["snap-mid".to_string(), "snap-old".to_string()]);
		StalwartSnapshot::prunable(&listing, "snap-new", 7)
			.is_empty()
			.xpect_true();
	}

	/// An incomplete snapshot is not a backup, so it neither occupies a slot in
	/// the window nor is deleted out from under the run that is creating it.
	#[beet_core::test]
	fn only_completed_snapshots_are_counted_and_the_new_one_is_never_deleted() {
		let listing = listing();
		StalwartSnapshot::prunable(&listing, "snap-new", 1)
			.join(",")
			.as_str()
			.xnot()
			.xpect_contains("snap-pending")
			.xnot()
			.xpect_contains("snap-new");
		// ..and a retain of zero prunes nothing rather than everything
		StalwartSnapshot::prunable(&listing, "snap-new", 0)
			.is_empty()
			.xpect_true();
	}
}
