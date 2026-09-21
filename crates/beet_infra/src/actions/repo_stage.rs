//! Staging the tree a deploy publishes, so one mirror publishes it whole.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<RepoStage src="site">` — assemble the tree this deploy publishes as its
/// document, in the staging dir [`RepoSync`] mirrors from.
///
/// A published document is rarely one checkout directory: the site's own tree
/// plus a few files built elsewhere in the workspace. Staging composes them
/// OUTSIDE the checkout, so no deploy step writes into source, and the staging
/// dir IS the store's contents, so a test compares the two directly.
///
/// `src` is copied whole (or its `paths` alone, for an entry at a checkout's
/// root whose siblings are crates and a `target/`), then the children run in
/// order: a [`DirCopy`] under a stage resolves its `dest` inside the staging
/// dir, so the borrow manifest is the same tag the local `assets` verb uses.
///
/// ```bsx
/// <RepoStage src="site">
///     <DirCopy src="assets" dest="assets" paths="wasm/beet-full.wasm"/>
/// </RepoStage>
/// <RepoSync/>
/// ```
///
/// The dir lives under the deploy work directory (`target/infra/<app>/repo`)
/// and is emptied first, so a file removed from the source leaves the mirror
/// rather than lingering from an earlier stage.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(
	BypassErrors = BypassErrors(
		ChildError::NO_ACTION | ChildError::ACTION_MISMATCH | ChildError::NONE_VALID
	)
)]
pub async fn RepoStage(
	/// The workspace-relative directory to publish.
	#[field]
	src: WsPath,
	/// Comma-separated paths under `src` to publish, each a file or a
	/// directory; empty publishes `src` whole. The allowlist for a document
	/// whose entry sits at a checkout's root (`paths="main.bsx,routes,templates"`),
	/// so the crates and the `target/` beside it never reach the store.
	#[field]
	paths: SmolStr,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let dir = cx
		.caller
		.with_state::<StackQuery, _>(|entity, stacks| {
			RepoStage::dir(&stacks, entity)
		})
		.await??;
	let src = src.into_abs();
	if fs_ext::is_dir_empty(&src)? {
		bevybail!("nothing to stage at {src}: the source is missing or empty");
	}
	// a fresh copy, so nothing from an earlier stage survives
	fs_ext::remove(&dir).ok();
	match paths.is_empty() {
		true => fs_ext::copy_recursive(&src, &dir)?,
		false => {
			DirCopy::copy_paths(&src, &dir, &paths)?;
		}
	}
	info!("staged {src} -> {dir}");

	// then the borrowed paths, each resolving the staging dir by ancestry
	let children = BehaviourChildren::valid_for::<
		Request,
		Outcome<Request, Response>,
	>(&cx.world(), cx.id(), "repo stage")
	.await?;
	let world = cx.world();
	let mut input = cx.input;
	for child in children {
		match world
			.entity(child)
			.call::<Request, Outcome<Request, Response>>(input)
			.await?
		{
			Outcome::Pass(next) => input = next,
			Outcome::Fail(response) => return Outcome::Fail(response).xok(),
		}
	}
	Outcome::Pass(input).xok()
}

impl RepoStage {
	/// A stage of the workspace-relative `src`, whole.
	pub fn new(src: impl Into<WsPath>) -> Self {
		Self {
			src: src.into(),
			paths: SmolStr::default(),
		}
	}

	/// A stage of the `paths` under `src` alone, see the `paths` field.
	pub fn with_paths(mut self, paths: impl Into<SmolStr>) -> Self {
		self.paths = paths.into();
		self
	}

	/// The staging dir of the stack `entity` belongs to: `repo/` under the
	/// deploy work directory, so a teardown removes it with the rest.
	pub fn dir(stacks: &StackQuery, entity: Entity) -> Result<AbsPath> {
		let (_, stack) = stacks.root(entity)?;
		stacks
			.deployment()
			.work_directory(&stack)
			.into_abs()
			.join("repo")
			.xok()
	}
}
