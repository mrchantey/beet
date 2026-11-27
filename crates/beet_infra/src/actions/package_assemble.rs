//! Assembling the tree a deploy publishes, so one mirror publishes it whole.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<PackageAssemble src="site">` — assemble the tree this deploy publishes as its
/// document, in the assembled copy [`PackageSync`] mirrors from.
///
/// A published document is rarely one checkout directory: the site's own tree
/// plus a few files owned elsewhere in the workspace. The assemble composes them
/// OUTSIDE the checkout, so no deploy step writes into source, and the
/// assembled copy IS the store's contents, so a test compares the two directly. A build
/// output the document serves is never borrowed: a [`WasmRuntime`] declared
/// before the assemble builds it, and the assemble reads what was just built.
///
/// `src` is copied whole (or its `paths` alone, for an entry at a checkout's
/// root whose siblings are crates and a `target/`), then the children run in
/// order: a [`DirCopy`] under an assemble resolves its `dest` inside the
/// assembled copy, so the borrow manifest is the same tag the local `assets` verb uses,
/// and a [`Derive`] ensures the document's derived files there, after the
/// borrows a derivation may read.
///
/// ```bsx
/// <PackageAssemble src="site">
///     <DirCopy src="assets" dest="assets" paths="databases/country.mmdb"/>
///     <Derive/>
/// </PackageAssemble>
/// <PackageSync/>
/// ```
///
/// The dir lives under the deploy work directory (`target/infra/<app>/package`)
/// and is emptied first, so a file removed from the source leaves the mirror
/// rather than lingering from an earlier assemble. A declared secrets document
/// sitting in `src` ([`Secrets::within_store`], an entry's conventional
/// `secrets.toml` beside it) is left out of the copy: sealed or not, it never
/// travels with the tree it belongs to.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(
	BypassErrors = BypassErrors(
		ChildError::NO_ACTION | ChildError::ACTION_MISMATCH | ChildError::NONE_VALID
	)
)]
pub async fn PackageAssemble(
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
			PackageAssemble::dir(&stacks, entity)
		})
		.await??;
	let src = src.into_abs();
	if fs_ext::is_dir_empty(&src)? {
		bevybail!(
			"nothing to assemble at {src}: the source is missing or empty"
		);
	}
	// a fresh copy, so nothing from an earlier assemble survives
	fs_ext::remove(&dir).ok();
	match paths.is_empty() {
		true => fs_ext::copy_recursive(&src, &dir)?,
		false => {
			DirCopy::copy_paths(&src, &dir, &paths)?;
		}
	}
	let documents = PackageAssemble::secrets_documents(&cx.caller).await?;
	for document in PackageAssemble::leave_out(&dir, &documents)? {
		info!("left the secrets document {document} out of the assembled copy");
	}
	info!("assembled {src} -> {dir}");

	// then the borrowed paths, each resolving the assembled copy by ancestry
	let children = BehaviourChildren::valid_for::<
		Request,
		Outcome<Request, Response>,
	>(&cx.world(), cx.id(), "package assemble")
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

impl PackageAssemble {
	/// An assemble of the workspace-relative `src`, whole.
	pub fn new(src: impl Into<WsPath>) -> Self {
		Self {
			src: src.into(),
			paths: SmolStr::default(),
		}
	}

	/// An assemble of the `paths` under `src` alone, see the `paths` field.
	pub fn with_paths(mut self, paths: impl Into<SmolStr>) -> Self {
		self.paths = paths.into();
		self
	}

	/// The path of every declared secrets document within its store, which
	/// an assemble of that store's directory leaves out.
	async fn secrets_documents(caller: &AsyncEntity) -> Result<Vec<SmolStr>> {
		caller
			.with_state::<Query<&Secrets>, _>(|_, documents| {
				documents
					.iter()
					.filter_map(Secrets::within_store)
					.map(SmolStr::from)
					.collect::<Vec<_>>()
			})
			.await
	}

	/// Remove each of `documents` from the assembled copy at `dir`, answering
	/// the ones it held.
	fn leave_out(dir: &AbsPath, documents: &[SmolStr]) -> Result<Vec<SmolStr>> {
		let mut removed = Vec::new();
		for document in documents {
			let copied = dir.join(document.as_str());
			if fs_ext::exists(&copied)? {
				fs_ext::remove(&copied)?;
				removed.push(document.clone());
			}
		}
		removed.xok()
	}

	/// The assembled copy of the stack `entity` belongs to: `package/` under the
	/// deploy work directory, so a teardown removes it with the rest.
	pub fn dir(stacks: &StackQuery, entity: Entity) -> Result<AbsPath> {
		let (_, stack) = stacks.root(entity)?;
		stacks
			.deployment()
			.work_directory(&stack)?
			.into_abs()
			.join("package")
			.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// An entry's conventional `secrets.toml`, copied with the rest of its
	/// directory, is left out of the assembled copy; nothing else is.
	///
	/// REGRESSION: `examples/beetmania` gained its own document beside its
	/// `main.bsx`, and the deploy's assemble would have published it into the
	/// package bucket the function serves.
	#[beet_core::test]
	fn leaves_the_secrets_document_out() {
		let dir = TempDir::new().unwrap();
		let root = AbsPath::new(dir.path()).unwrap();
		fs_ext::write(root.join("main.bsx"), "<Router/>").unwrap();
		fs_ext::write(root.join("secrets.toml"), "sealed").unwrap();
		let documents = [Secrets::default(), Secrets::new("../secrets.toml")]
			.iter()
			.filter_map(Secrets::within_store)
			.map(SmolStr::from)
			.collect::<Vec<_>>();
		PackageAssemble::leave_out(&root, &documents)
			.unwrap()
			.xpect_eq(vec![SmolStr::from("secrets.toml")]);
		fs_ext::exists(root.join("secrets.toml"))
			.unwrap()
			.xpect_false();
		fs_ext::exists(root.join("main.bsx")).unwrap().xpect_true();
	}
}
