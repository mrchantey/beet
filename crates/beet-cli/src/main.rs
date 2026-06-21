//! The `beet` binary: discover an entry, supply the process request, load it,
//! let the loaded tree run itself, and exit unless something kept it alive.
//!
//! beet is unopinionated like a game engine: it links a library of capabilities
//! (registered reflect types) but ships zero behaviour. It discovers `main.bsx`
//! (or `main.json`/`main.ron`) by walking the cwd and its ancestors, consumes
//! only its own `--main` flag, and builds the entry through the unified
//! [`TemplateLoader`], then lets the `BootOnLoad` verb fan the process request out
//! on the build's `LoadTemplate`. A one-shot streams its response and exits; a
//! long-running server parks its boot call to persist the process.
// the wasm build is a Cloudflare Worker `cdylib` (see `lib.rs`'s `#[event(fetch)]`
// entry, built with `--lib`); the bin target keeps an empty `main` so it still
// links if ever built for wasm.
#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
use beet::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use beet_cli::prelude::*;

/// Entry-document file names discovery looks for, in priority order, walking the
/// cwd and its ancestors (a `Cargo.toml`-style walk).
#[cfg(not(target_arch = "wasm32"))]
const ENTRY_NAMES: &[&str] = &["main.bsx", "main.json", "main.ron"];

#[cfg(not(target_arch = "wasm32"))]
fn main() -> AppExit {
	// load any local `.env` (eg `BEET_REMOTE_URL`) before the app starts.
	env_ext::load_dotenv();

	let mut app = App::new();
	// the cross-platform serve plugins, shared with the wasm Worker entry.
	add_serve_plugins(&mut app);
	// the native-only dev-command and terminal targets layered on top.
	add_native_serve_plugins(&mut app);

	// the process exits when `boot` writes `AppExit` for the one-shot it
	// resolves; a long-running server parks its boot call, so its unresolved
	// `Running<Response>` persists the process with no refcount.
	app.add_systems(Startup, load_entry).run()
}

/// `Startup`: resolve the entry and build it through the unified [`TemplateLoader`].
/// The build fires `LoadTemplate` on the root, where the `BootOnLoad` verb fans the
/// process request out to the entry's servers. The app loop then drives that boot
/// and stays alive until it writes `AppExit`, so nothing is held by hand here. A
/// failed build logs and exits with an error rather than panicking. Run here (not
/// before the app) so the message goes through the initialized logger.
#[cfg(not(target_arch = "wasm32"))]
fn load_entry(world: &mut World) {
	if let Err(err) = try_load_entry(world) {
		error!("{err}");
		world.write_message(AppExit::error());
	}
}

#[cfg(not(target_arch = "wasm32"))]
fn try_load_entry(world: &mut World) -> Result {
	// resolve the site store + the entry document name within it: the filesystem
	// locally, an S3 bucket in a deployed task. The store roots the whole site, so
	// the entry document, `templates/`, `<RoutesDir/>` and `<Template src>` includes
	// all load through the one [`BlobStore`].
	let (site_root, entry_name) = resolve_site_root()?;
	site_root.register_templates(world)?;
	let media = async_ext::block_on(
		site_root.0.get_media(&SmolPath::from(entry_name.as_str())),
	)?;
	world.insert_resource(site_root);

	TemplateLoader::new(world).load(&media).map_err(|err| {
		bevyhow!("failed to load entry `{entry_name}`: {err}")
	})?;
	Ok(())
}

/// Resolve the [`SiteRoot`] store and the entry document name within it.
///
/// A deployed task (`BEET_SERVICE_ACCESS=remote`) loads the site from its S3
/// bucket; otherwise discovery walks the filesystem for a local `main.bsx`.
#[cfg(not(target_arch = "wasm32"))]
fn resolve_site_root() -> Result<(SiteRoot, String)> {
	// remote: pull the whole site from the S3 bucket the deploy injected.
	#[cfg(feature = "aws_sdk")]
	if remote_access() {
		return remote_site_root();
	}

	// local: the binary's own `--main` overrides discovery; the loaded tree
	// re-parses argv itself, so the binary consumes only its own `--main` here.
	let mut args = CliArgs::parse_env();
	let entry = match args
		.params
		.remove("main")
		.and_then(|values| values.into_iter().next())
	{
		Some(path) => AbsPathBuf::new(path.as_str())?,
		None => discover_entry()?,
	};
	let dir = entry
		.parent()
		.ok_or_else(|| bevyhow!("entry `{entry}` has no parent directory"))?;
	let entry_name = entry
		.file_name()
		.and_then(|name| name.to_str())
		.ok_or_else(|| bevyhow!("entry `{entry}` has no file name"))?
		.to_string();
	Ok((SiteRoot::new_fs(dir), entry_name))
}

/// Whether the runtime should access services remotely (the deployed task), read
/// from `BEET_SERVICE_ACCESS`.
#[cfg(feature = "aws_sdk")]
fn remote_access() -> bool {
	env_ext::var("BEET_SERVICE_ACCESS")
		.map(|value| value.eq_ignore_ascii_case("remote"))
		.unwrap_or(false)
}

/// A [`SiteRoot`] backed by the deploy's S3 site bucket (`BEET_SITE_BUCKET`); the
/// entry document is `main.bsx` at the bucket root.
///
/// An explicit `BEET_S3_ENDPOINT` (eg `https://<account>.r2.cloudflarestorage.com`)
/// switches the store onto an S3-compatible service such as Cloudflare R2: the
/// region becomes `auto`, path-style addressing is used, and the same `AWS_*`
/// keys carry the R2 credentials. Unset, it reads AWS S3 in `AWS_REGION`. So one
/// container binary serves identically on Fargate (S3) and Cloudflare (R2).
#[cfg(feature = "aws_sdk")]
fn remote_site_root() -> Result<(SiteRoot, String)> {
	let bucket = env_ext::var("BEET_SITE_BUCKET").map_err(|_| {
		bevyhow!("BEET_SERVICE_ACCESS=remote but BEET_SITE_BUCKET is unset")
	})?;
	let store = match env_ext::var("BEET_S3_ENDPOINT") {
		Ok(endpoint) => {
			info!("loading site from r2/s3 bucket `{bucket}` ({endpoint})");
			S3Store::new(bucket, "auto").with_endpoint(endpoint)
		}
		Err(_) => {
			let region = env_ext::var("AWS_REGION")
				.unwrap_or_else(|_| "us-west-2".to_string());
			info!("loading site from s3 bucket `{bucket}` ({region})");
			S3Store::new(bucket, region)
		}
	};
	Ok((SiteRoot(BlobStore::new(store)), "main.bsx".to_string()))
}

/// Walk the cwd and its ancestors for the first [`ENTRY_NAMES`] match, erroring
/// with guidance when none is found.
#[cfg(not(target_arch = "wasm32"))]
fn discover_entry() -> Result<AbsPathBuf> {
	let start = AbsPathBuf::new(".")?;
	let mut dir = Some(start.clone());
	while let Some(current) = dir {
		for name in ENTRY_NAMES {
			let candidate = current.join(name);
			if fs_ext::exists(&candidate)? {
				return Ok(candidate);
			}
		}
		dir = current.parent();
	}
	bevybail!(
		"no entry document found: looked for {ENTRY_NAMES:?} in `{start}` and its \
		ancestors. Create a `main.bsx` or pass `--main=<path>`."
	)
}
