//! Cloudflare deploy actions, driven by the `wrangler` CLI as `ChildProcess`
//! `#[action]`s (mirroring [`BuildDockerImageAction`]). Used by the
//! [`CloudflareContainerBlock`] and [`CloudflareWorkerBlock`] examples.
//!
//! `wrangler` is the deploy tool (not OpenTofu): it has first-class
//! `r2 bucket create/delete`, `deploy`, `delete` and `tail`, and natively builds
//! + pushes a container image (or runs `worker-build` for a wasm Worker) on
//! `deploy`. The `cf` CLI is a thinner JSON-over-REST wrapper and is the
//! documented fallback.
//!
//! Live deploy needs `CLOUDFLARE_API_TOKEN` (+ `CLOUDFLARE_ACCOUNT_ID`) in the
//! environment and, for the container path, the R2 data-plane keys
//! (`R2_ACCESS_KEY_ID` / `R2_SECRET_ACCESS_KEY`) so the container reads the site
//! via [`S3Store::r2`]. The Worker path needs neither (native `worker::Bucket`
//! binding). All commands are `--dry-run`-able; see each example's module doc.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Default Workers compatibility date stamped into generated `wrangler.jsonc`.
const COMPATIBILITY_DATE: &str = "2025-06-01";

/// Where the `build` verb publishes the deployable wasm Worker artifacts
/// (`index.js`, `index_bg.wasm`, `package.json`), workspace-relative. `deploy`
/// points the generated `wrangler.jsonc` `main` here, so the upload reuses the
/// build output instead of recompiling. Under `assets/` so it is gitignored.
const WORKER_ASSETS_DIR: &str = "assets/worker";

/// The container Durable Object class name. Cloudflare derives the deployed
/// container application name as `<worker-name>-<lowercased class name>`
/// (eg `beet-hello-container-beetcontainer`) and names the managed-registry image
/// repository the same, which the teardown matches on to delete both.
const CONTAINER_CLASS: &str = "BeetContainer";

/// The container application + image repository name Cloudflare derives for a
/// given worker name (`<worker>-<lowercased class>`). Both the container app and
/// its registry image use this, so teardown lists by name and deletes both.
fn container_app_name(worker_name: &str) -> String {
	format!("{worker_name}-{}", CONTAINER_CLASS.to_lowercase())
}

// ───────────────────────────── shared wrangler helpers ─────────────────────

/// Create an R2 bucket, treating an "already exists" failure as success so
/// `deploy` is idempotent.
async fn wrangler_r2_create(bucket: &str) -> Result {
	info!("ensuring R2 bucket `{bucket}`");
	// `run_async` errors on a non-zero exit, folding wrangler's stderr into the
	// error message, so match on that: `10004` / "already exists" / "already
	// owned" are the idempotent cases (the bucket is already there and ours).
	match ChildProcess::new("wrangler")
		.with_args(["r2", "bucket", "create", bucket])
		.run_async()
		.await
	{
		Ok(_) => Ok(()),
		Err(err) => {
			let message = err.to_string();
			if message.contains("already") || message.contains("10004") {
				info!("R2 bucket `{bucket}` already exists");
				Ok(())
			} else {
				Err(err)
			}
		}
	}
}

/// `wrangler deploy` from the given project directory. When `secrets_file` is
/// set, its keys are uploaded as real Worker secrets *with* this version
/// (`--secrets-file`), the only way a deploy publishes secrets: a `.dev.vars`
/// file is a local-dev artifact `wrangler deploy` otherwise ignores. The
/// container path needs this so its `this.env.R2_*` reads resolve in production.
async fn wrangler_deploy(
	project_dir: &AbsPathBuf,
	secrets_file: Option<&str>,
) -> Result {
	info!("wrangler deploy ({})", project_dir.display());
	let mut args = vec!["deploy".to_string()];
	if let Some(secrets_file) = secrets_file {
		args.push("--secrets-file".to_string());
		args.push(secrets_file.to_string());
	}
	ChildProcess::new("wrangler")
		.with_args(args)
		.with_cwd(project_dir.clone())
		.run_async()
		.await?;
	Ok(())
}

/// The build directory for a Cloudflare project (`target/<name>-cf/`), created
/// fresh.
fn cf_project_dir(name: &str) -> Result<AbsPathBuf> {
	let dir = AbsPathBuf::new_workspace_rel(".")?
		.join("target")
		.join(format!("{name}-cf"));
	fs_ext::create_dir_all(&dir)?;
	Ok(dir)
}

/// Find a sibling component of type `T` by walking the action's parent's children
/// (the same pattern [`BuildDockerImageAction`] uses for its block + artifact).
async fn sibling<T: Component + Clone>(
	cx: &ActionContext<Request>,
) -> Result<T> {
	cx.caller
		.with_state::<(Query<&Children>, Query<&ChildOf>, Query<&T>), _>(
			|entity, (children_q, child_of_q, comp_q)| -> Result<T> {
				let parent = child_of_q
					.get(entity)
					.map(|child_of| child_of.parent())
					.map_err(|_| bevyhow!("deploy action has no parent"))?;
				let children = children_q
					.get(parent)
					.map_err(|_| bevyhow!("parent has no children"))?;
				children
					.iter()
					.find_map(|child| comp_q.get(child).ok().cloned())
					.ok_or_else(|| {
						bevyhow!(
							"no sibling {} found",
							core::any::type_name::<T>()
						)
					})
			},
		)
		.await?
}

// ───────────────────────────── container deploy ────────────────────────────

/// Deploy the native `beet` binary to Cloudflare Containers: build the project
/// (Dockerfile + worker shim + `wrangler.jsonc`), ensure the R2 bucket, then
/// `wrangler deploy` (which builds + pushes the image to Cloudflare's managed
/// registry and deploys the fronting Worker). Reads the sibling
/// [`CloudflareContainerBlock`] + [`BuildArtifact`].
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn CloudflareContainerDeployAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let block = sibling::<CloudflareContainerBlock>(&cx).await?;
	let artifact = sibling::<BuildArtifact>(&cx).await?;

	let binary = AbsPathBuf::new(artifact.artifact_path())?;
	if !binary.exists() {
		bevybail!("binary not found at: {}", binary.display());
	}

	// the R2 endpoint the container's `S3Store::r2` reads through; the account id
	// is also needed to address the managed registry on deploy.
	let account_id = env_ext::var("CLOUDFLARE_ACCOUNT_ID")
		.map_err(|_| bevyhow!("CLOUDFLARE_ACCOUNT_ID is unset"))?;
	let endpoint = format!("https://{account_id}.r2.cloudflarestorage.com");

	let dir = cf_project_dir(block.name())?;
	let binary_name = "beet";
	std::fs::copy(&binary, dir.join(binary_name))?;
	write_container_dockerfile(&dir, binary_name, block.port())?;
	write_container_worker_js(&dir, &block, &endpoint)?;
	write_container_wrangler(&dir, &block)?;
	write_container_package_json(&dir)?;
	let secrets_file = write_r2_secrets_file(&dir)?;

	// `wrangler deploy` bundles `worker.js`, whose `@cloudflare/containers` import
	// is resolved from `node_modules`, so install deps before deploying.
	npm_install(&dir).await?;
	wrangler_r2_create(block.bucket()).await?;
	// upload the R2 keys as real Worker secrets with this version (`.dev.vars` is
	// otherwise local-only), so the container's `this.env.R2_*` reads resolve.
	wrangler_deploy(&dir, secrets_file.as_deref()).await?;
	info!("deployed container worker `{}`", block.name());
	Pass(cx.input).xok()
}

/// The Dockerfile: the native `beet` binary on debian-slim, serving http on the
/// container port. The site is pulled from R2 at boot, not baked in.
fn write_container_dockerfile(
	dir: &AbsPathBuf,
	binary_name: &str,
	port: u16,
) -> Result {
	// the port is driven by the served site's markup `HttpServer{port}` (the
	// binary loads it from R2 at boot), so the container only needs to EXPOSE it;
	// `--server=http --path=/` mirrors the proven Fargate invocation.
	let dockerfile = format!(
		"FROM debian:bookworm-slim\n\
		 RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*\n\
		 COPY {binary_name} /app\n\
		 RUN chmod +x /app\n\
		 EXPOSE {port}\n\
		 CMD [\"/app\", \"serve\", \"--server=http\", \"--path=/\"]\n"
	);
	fs_ext::write(dir.join("Dockerfile"), dockerfile)?;
	Ok(())
}

/// The fronting Worker: a `Container` Durable Object that proxies every request
/// to the container, injecting the remote-store env (the R2 creds come from the
/// Worker's secrets, the rest from `vars`).
fn write_container_worker_js(
	dir: &AbsPathBuf,
	block: &CloudflareContainerBlock,
	endpoint: &str,
) -> Result {
	let port = block.port();
	let sleep_after = block.sleep_after();
	let bucket = block.bucket();
	// non-secret env as literals; secrets (R2 keys) read from `this.env` at runtime.
	// `BEET_HOST=0.0.0.0` binds the server to all interfaces (matching Fargate /
	// Lightsail): the fronting Worker proxies to the container's own IP, so a
	// default localhost bind would be unreachable ("not listening in the TCP
	// address <ip>:<port>").
	let mut env_lines = format!(
		"    BEET_SERVICE_ACCESS: \"remote\",\n\
		 \x20   BEET_HOST: \"0.0.0.0\",\n\
		 \x20   BEET_SITE_BUCKET: \"{bucket}\",\n\
		 \x20   BEET_S3_ENDPOINT: \"{endpoint}\",\n\
		 \x20   AWS_ACCESS_KEY_ID: this.env.R2_ACCESS_KEY_ID,\n\
		 \x20   AWS_SECRET_ACCESS_KEY: this.env.R2_SECRET_ACCESS_KEY,\n"
	);
	for var in block.env_vars() {
		env_lines.push_str(&format!(
			"    {}: this.env.{},\n",
			var.key(),
			var.key()
		));
	}
	let js = format!(
		"import {{ Container, getContainer }} from \"@cloudflare/containers\";\n\
		 \n\
		 export class {CONTAINER_CLASS} extends Container {{\n\
		 \x20 defaultPort = {port};\n\
		 \x20 sleepAfter = \"{sleep_after}\";\n\
		 \x20 envVars = {{\n{env_lines}  }};\n\
		 }}\n\
		 \n\
		 export default {{\n\
		 \x20 async fetch(request, env) {{\n\
		 \x20   return getContainer(env.BEET_CONTAINER).fetch(request);\n\
		 \x20 }},\n\
		 }};\n"
	);
	fs_ext::write(dir.join("worker.js"), js)?;
	Ok(())
}

/// Version of `@cloudflare/containers` the generated worker imports.
const CONTAINERS_PKG_VERSION: &str = "^0.3.7";

/// Write the `package.json` declaring `@cloudflare/containers`, which the
/// generated `worker.js` imports. Wrangler's bundler resolves this from
/// `node_modules` on `deploy`, so without it the deploy fails with
/// `Could not resolve "@cloudflare/containers"`.
fn write_container_package_json(dir: &AbsPathBuf) -> Result {
	let json = serde_json::to_string_pretty(&serde_json::json!({
		"name": "beet-container-worker",
		"private": true,
		"dependencies": { "@cloudflare/containers": CONTAINERS_PKG_VERSION },
	}))?;
	fs_ext::write(dir.join("package.json"), json)?;
	Ok(())
}

/// `npm install` in the project dir, populating `node_modules` so wrangler can
/// bundle the worker's npm imports. Quiet + no audit/fund noise.
async fn npm_install(dir: &AbsPathBuf) -> Result {
	info!("npm install ({})", dir.display());
	ChildProcess::new("npm")
		.with_args(["install", "--no-audit", "--no-fund", "--loglevel=error"])
		.with_cwd(dir.clone())
		.run_async()
		.await?;
	Ok(())
}

/// `wrangler.jsonc` binding the container Durable Object + the R2 bucket.
fn write_container_wrangler(
	dir: &AbsPathBuf,
	block: &CloudflareContainerBlock,
) -> Result {
	let json = serde_json::to_string_pretty(&serde_json::json!({
		"name": block.name(),
		"main": "worker.js",
		"compatibility_date": COMPATIBILITY_DATE,
		"containers": [{
			"class_name": CONTAINER_CLASS,
			"image": "./Dockerfile",
			"max_instances": block.max_instances(),
			// the smallest instance; wrangler 4.103 renamed the former "dev" to "lite".
			"instance_type": "lite",
		}],
		"durable_objects": {
			"bindings": [{ "name": "BEET_CONTAINER", "class_name": CONTAINER_CLASS }],
		},
		"migrations": [{ "tag": "v1", "new_sqlite_classes": [CONTAINER_CLASS] }],
	}))?;
	fs_ext::write(dir.join("wrangler.jsonc"), json)?;
	Ok(())
}

/// Write the R2 data-plane keys to a `.env`-format secrets file (`secrets.env`)
/// the deploy uploads as real Worker secrets (`wrangler deploy --secrets-file`).
/// Returns the file name (relative to the project dir, which is the deploy cwd),
/// or `None` when the keys are absent so a dry run still works.
fn write_r2_secrets_file(dir: &AbsPathBuf) -> Result<Option<String>> {
	match (
		env_ext::var("R2_ACCESS_KEY_ID"),
		env_ext::var("R2_SECRET_ACCESS_KEY"),
	) {
		(Ok(id), Ok(secret)) => {
			let file_name = "secrets.env";
			fs_ext::write(
				dir.join(file_name),
				format!(
					"R2_ACCESS_KEY_ID={id}\nR2_SECRET_ACCESS_KEY={secret}\n"
				),
			)?;
			Some(file_name.to_string()).xok()
		}
		_ => {
			warn!(
				"R2_ACCESS_KEY_ID / R2_SECRET_ACCESS_KEY unset; the container \
				 cannot read R2 until they are uploaded as Worker secrets"
			);
			None.xok()
		}
	}
}

// ───────────────────────────── worker build ────────────────────────────────

/// Compile the wasm Worker and publish its artifacts to [`WORKER_ASSETS_DIR`]
/// without deploying. The slow step (a full wasm-bindgen + wasm-opt build) that
/// `deploy` and `bench` reuse; running it as its own verb makes the artifacts
/// (and the wasm size) visible before an upload, and warms the build so a
/// following `deploy` only uploads.
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn CloudflareWorkerBuildAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let start = Instant::now();
	let wasm_size = build_worker_artifacts().await?;
	info!(
		"built worker: {} wasm in {} (published to {WORKER_ASSETS_DIR}/)",
		fmt_bytes(wasm_size),
		time_ext::pretty_print_duration(start.elapsed()),
	);
	Pass(cx.input).xok()
}

/// Compile the `beet-cli` crate to a wasm Worker with `worker-build` and copy the
/// deployable artifacts into [`WORKER_ASSETS_DIR`], returning the wasm size in
/// bytes. `worker-build` wraps wasm-bindgen + wasm-opt and emits `index.js` +
/// `index_bg.wasm` under `<crate>/build/`; the copy makes them inspectable and
/// lets `deploy` upload them without a rebuild.
async fn build_worker_artifacts() -> Result<u64> {
	let cli_dir = AbsPathBuf::new_workspace_rel("crates/beet-cli")?;
	let cli_arg = cli_dir.to_string();
	info!("building wasm worker (worker-build --release)");
	ChildProcess::new("worker-build")
		.with_args([
			"--release",
			cli_arg.as_str(),
			"--",
			"--no-default-features",
			"--features",
			"cloudflare",
		])
		.run_async()
		.await?;
	// worker-build emits `<crate>/build/{index.js,index_bg.wasm,package.json}`;
	// the `worker/shim.mjs` it also writes is a backwards-compat re-export of
	// `index.js`, so these three files are the whole deployable set.
	let build_dir = cli_dir.join("build");
	let assets_dir = AbsPathBuf::new_workspace_rel(WORKER_ASSETS_DIR)?;
	let mut wasm_size = 0;
	for name in ["index.js", "index_bg.wasm", "package.json"] {
		let bytes = fs_ext::copy(build_dir.join(name), assets_dir.join(name))?;
		if name == "index_bg.wasm" {
			wasm_size = bytes;
		}
	}
	Ok(wasm_size)
}

/// Ensure the prebuilt Worker artifacts exist, building them first if the `build`
/// verb has not run, so a bare `deploy` still works.
async fn ensure_worker_artifacts() -> Result {
	let index =
		AbsPathBuf::new_workspace_rel(WORKER_ASSETS_DIR)?.join("index.js");
	if !index.exists() {
		info!("no prebuilt worker at {WORKER_ASSETS_DIR}/, building first");
		build_worker_artifacts().await?;
	}
	Ok(())
}

/// Format a byte count as a human-readable size, eg `17.4 MB`.
fn fmt_bytes(bytes: u64) -> String {
	const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
	let mut size = bytes as f64;
	let mut unit = 0;
	while size >= 1024.0 && unit < UNITS.len() - 1 {
		size /= 1024.0;
		unit += 1;
	}
	format!("{size:.1} {}", UNITS[unit])
}

// ───────────────────────────── worker deploy ───────────────────────────────

/// Deploy `beet-cli` (wasm) as a Cloudflare Worker: ensure the prebuilt artifacts
/// (from `build`, or built now), write `wrangler.jsonc` pointing `main` at them
/// plus the R2 binding, ensure the R2 bucket, then `wrangler deploy` (upload, no
/// recompile). Reads the sibling [`CloudflareWorkerBlock`].
///
/// The wasm artifact is produced from the `beet-cli` crate (`--features
/// cloudflare`) by `worker-build` in [`build_worker_artifacts`], so this action
/// carries no separate [`BuildArtifact`].
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn CloudflareWorkerDeployAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let start = Instant::now();
	let block = sibling::<CloudflareWorkerBlock>(&cx).await?;
	ensure_worker_artifacts().await?;
	let dir = cf_project_dir(block.name())?;
	write_worker_wrangler(&dir, &block)?;

	wrangler_r2_create(block.bucket()).await?;
	wrangler_deploy(&dir, None).await?;
	info!(
		"deployed wasm worker `{}` in {}",
		block.name(),
		time_ext::pretty_print_duration(start.elapsed()),
	);
	Pass(cx.input).xok()
}

/// `wrangler.jsonc` for the wasm Worker: `main` points at the prebuilt artifacts
/// (no `build.command`, so the deploy uploads them as-is), plus the R2 bucket
/// bound by [`WORKER_R2_BINDING`].
fn write_worker_wrangler(
	dir: &AbsPathBuf,
	block: &CloudflareWorkerBlock,
) -> Result {
	// `main` is the prebuilt `index.js` (the wasm-bindgen entry; its `index_bg.wasm`
	// sibling resolves by relative import). An absolute path outside this wrangler
	// project dir is fine: wrangler bundles `main` and follows its wasm import.
	let main_js =
		AbsPathBuf::new_workspace_rel(WORKER_ASSETS_DIR)?.join("index.js");
	let vars = block
		.env_vars()
		.iter()
		.map(|var| (var.key().to_string(), var.key().to_string()))
		.collect::<std::collections::BTreeMap<_, _>>();
	let json = serde_json::to_string_pretty(&serde_json::json!({
		"name": block.name(),
		"main": main_js.to_string(),
		"compatibility_date": COMPATIBILITY_DATE,
		"compatibility_flags": ["nodejs_compat"],
		"r2_buckets": [{
			"binding": WORKER_R2_BINDING,
			"bucket_name": block.bucket(),
		}],
		"vars": vars,
	}))?;
	fs_ext::write(dir.join("wrangler.jsonc"), json)?;
	Ok(())
}

// ───────────────────────────── R2 site sync ────────────────────────────────

/// Publishes a local site directory to an R2 bucket, key-free: it walks the
/// directory and runs `wrangler r2 object put` per file (using the API token, so
/// no R2 S3 keys are needed for the sync itself). Read by
/// [`CloudflareR2SyncAction`].
#[derive(Debug, Clone, Default, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(CloudflareR2SyncAction)]
pub struct CloudflareR2Sync {
	/// Local directory to publish (workspace-relative), eg `examples/bsx_site`.
	local_dir: SmolPath,
	/// Target R2 bucket.
	bucket: SmolStr,
}

impl CloudflareR2Sync {
	/// Publish `local_dir` to `bucket`.
	pub fn new(
		local_dir: impl Into<SmolPath>,
		bucket: impl Into<SmolStr>,
	) -> Self {
		Self {
			local_dir: local_dir.into(),
			bucket: bucket.into(),
		}
	}
}

/// Walk the [`CloudflareR2Sync`] directory and upload each file to R2, timing the
/// publish (the headline the `bench` verb measures against a full redeploy).
#[action]
#[derive(Default, Component)]
pub async fn CloudflareR2SyncAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let start = Instant::now();
	let sync = cx.caller.get_cloned::<CloudflareR2Sync>().await?;
	sync_dir_to_r2(sync.local_dir().as_str(), sync.bucket()).await?;
	info!(
		"synced site in {} (live on the next fetch)",
		time_ext::pretty_print_duration(start.elapsed()),
	);
	Pass(cx.input).xok()
}

/// Upload every file under `local_dir` to `bucket` via `wrangler r2 object put`.
/// Shared by [`CloudflareR2SyncAction`] and [`CloudflareBenchAction`].
///
/// `local_dir` is resolved relative to the cwd (like `--main`), not the workspace:
/// the site is the user's, and a deploy `.bsx` may be run from a different repo
/// than the beet workspace that holds the Worker source.
async fn sync_dir_to_r2(local_dir: &str, bucket: &str) -> Result {
	let root = AbsPathBuf::new(local_dir)?;
	let files = ReadDir::files_recursive(&root)?;
	info!(
		"syncing {} files from {} to r2://{bucket}",
		files.len(),
		root.display(),
	);
	// precompute owned `(bucket/key, file)` args so the uploads own their data.
	let puts = files
		.into_iter()
		.map(|file| {
			let rel = file.strip_prefix(&root).unwrap_or(file.as_path());
			let key = rel.to_string_lossy().replace('\\', "/");
			(format!("{bucket}/{key}"), file.to_string_lossy().to_string())
		})
		.collect::<Vec<_>>();
	// each `wrangler r2 object put` is its own node process, so the per-file startup
	// dominates; upload concurrently. Bound the fan-out to 16: a large site (hundreds
	// of files) would otherwise spawn hundreds of concurrent node processes and
	// exhaust file descriptors / PIDs. 16 at a time keeps the wall-clock near one put.
	for chunk in puts.chunks(16) {
		chunk
			.iter()
			.map(|(key, file_arg)| async move {
				// `--remote` targets the real R2 bucket; without it wrangler writes to
				// its *local* Miniflare store, which a deployed Worker never reads.
				ChildProcess::new("wrangler")
					.with_args([
						"r2",
						"object",
						"put",
						key.as_str(),
						"--file",
						file_arg.as_str(),
						"--remote",
					])
					.run_async()
					.await
					.map(|_| ())
			})
			.xmap(async_ext::try_join_all)
			.await?;
	}
	Ok(())
}

// ───────────────────────────── bench ───────────────────────────────────────

/// Benchmarks the two ways to change a deployed Worker's behavior: an R2 `sync`
/// (publish the site, served on the next fetch via the Worker's per-request
/// version check) versus a full Worker rebuild + redeploy. Prints a side-by-side
/// comparison: the headline of the infra demo.
#[derive(Debug, Clone, Default, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(CloudflareBenchAction)]
pub struct CloudflareBench {
	/// Worker name, redeployed to time the full-redeploy path.
	name: SmolStr,
	/// R2 bucket the site is published to.
	bucket: SmolStr,
	/// Local site directory published on the sync path.
	local_dir: SmolPath,
	/// Optional live Worker URL; when set, the sync path also polls it until it
	/// serves a 200, timing how soon the fresh site is live.
	#[set_with(unwrap_option)]
	url: Option<SmolStr>,
}

impl CloudflareBench {
	/// Bench publishing `local_dir` to `bucket` against redeploying `name`.
	pub fn new(
		name: impl Into<SmolStr>,
		bucket: impl Into<SmolStr>,
		local_dir: impl Into<SmolPath>,
	) -> Self {
		Self {
			name: name.into(),
			bucket: bucket.into(),
			local_dir: local_dir.into(),
			url: None,
		}
	}
}

/// Time an R2 `sync` (and, with a `url`, how soon the live Worker serves it)
/// against a full rebuild + redeploy, then print the comparison.
#[action]
#[derive(Default, Component)]
pub async fn CloudflareBenchAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let bench = cx.caller.get_cloned::<CloudflareBench>().await?;

	// sync path: publish the site to R2; the Worker serves it on the next fetch.
	let sync_start = Instant::now();
	sync_dir_to_r2(bench.local_dir().as_str(), bench.bucket()).await?;
	let sync_elapsed = sync_start.elapsed();

	// with a url, also time how soon the live Worker serves the fresh site.
	let live_elapsed = match bench.url() {
		Some(url) => Some(poll_until_ok(url.as_str(), sync_start).await?),
		None => None,
	};

	// redeploy path: a full wasm rebuild + Worker redeploy (the prebuilt artifacts
	// are rebuilt, then uploaded).
	let redeploy_start = Instant::now();
	build_worker_artifacts().await?;
	let block = CloudflareWorkerBlock::new(bench.name().clone())
		.with_bucket(bench.bucket().clone());
	let dir = cf_project_dir(bench.name())?;
	write_worker_wrangler(&dir, &block)?;
	wrangler_deploy(&dir, None).await?;
	let redeploy_elapsed = redeploy_start.elapsed();

	let speedup = redeploy_elapsed.as_secs_f64() / sync_elapsed.as_secs_f64();
	cross_log!("\nupdate worker behavior — sync vs full redeploy");
	cross_log!(
		"  sync (R2 publish):       {}",
		time_ext::pretty_print_duration(sync_elapsed)
	);
	if let Some(live_elapsed) = live_elapsed {
		cross_log!(
			"  first fresh fetch:       {}",
			time_ext::pretty_print_duration(live_elapsed)
		);
	}
	cross_log!(
		"  full rebuild + redeploy: {}",
		time_ext::pretty_print_duration(redeploy_elapsed)
	);
	cross_log!("  → sync is {speedup:.0}x faster\n");
	Pass(cx.input).xok()
}

/// Poll `url` until it serves a 200, returning how long after `since` that took.
/// Bounded so an unreachable Worker fails the bench instead of hanging.
async fn poll_until_ok(url: &str, since: Instant) -> Result<Duration> {
	for _ in 0..100 {
		if let Ok(res) = Request::get(url).send().await
			&& res.status().is_ok()
		{
			return since.elapsed().xok();
		}
		time_ext::sleep(Duration::from_millis(100)).await;
	}
	bevybail!("worker at {url} did not serve a 200 within the bench window");
}

// ───────────────────────────── watch + destroy ─────────────────────────────

/// Polls a deployed Worker for readiness (the deploy + rollout is near-instant on
/// Cloudflare, unlike an ECS rollout). Reads the host (`<name>.workers.dev`) it
/// was constructed with.
#[derive(Debug, Clone, Default, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(CloudflareWatchAction)]
pub struct CloudflareWatch {
	/// Worker name, used to list deployments and (optionally) poll the host.
	name: SmolStr,
	/// Optional poll timeout.
	#[set_with(unwrap_option)]
	timeout: Option<Duration>,
}

impl CloudflareWatch {
	/// Watch the named Worker.
	pub fn new(name: impl Into<SmolStr>) -> Self {
		Self {
			name: name.into(),
			timeout: None,
		}
	}
}

/// Tail a deployed Worker's logs via `wrangler tail`, the Cloudflare analogue of
/// [`AwsWatchAction`]. With a timeout it tails then stops; otherwise it follows.
#[action]
#[derive(Default, Component)]
pub async fn CloudflareWatchAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let watch = cx.caller.get_cloned::<CloudflareWatch>().await?;
	info!("tailing worker `{}`", watch.name());
	// group: `wrangler` is a wrapper that spawns the real node process; a plain
	// kill orphans it and the leaked tail holds stdio open past process exit.
	let mut child = ChildProcess::new("wrangler")
		.with_args(["tail", watch.name().as_str(), "--format", "pretty"])
		.with_group(true)
		.spawn()?;
	if let Some(timeout) = watch.timeout() {
		time_ext::sleep(*timeout).await;
		child.kill().ok();
	} else {
		child.status().await?;
	}
	Pass(cx.input).xok()
}

/// Tears down a Cloudflare deploy: deletes the Worker and its R2 bucket. Reads
/// the sibling block (container or worker) for the names; mandatory for the
/// teardown gate.
#[derive(Debug, Clone, Default, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(CloudflareDestroyAction)]
pub struct CloudflareDestroy {
	/// Worker name to delete.
	name: SmolStr,
	/// R2 bucket to delete (after emptying).
	bucket: SmolStr,
	/// The site dir that was synced to the bucket (eg `examples/bsx_site`). When
	/// set, the action deletes those objects first, since `wrangler r2 bucket
	/// delete` refuses a non-empty bucket and has no `--force`/empty flag.
	#[set_with(unwrap_option)]
	local_dir: Option<SmolPath>,
}

impl CloudflareDestroy {
	/// Destroy the named Worker + bucket.
	pub fn new(name: impl Into<SmolStr>, bucket: impl Into<SmolStr>) -> Self {
		Self {
			name: name.into(),
			bucket: bucket.into(),
			local_dir: None,
		}
	}
}

/// `wrangler delete <worker>`, empty the bucket (deleting the synced objects),
/// then `wrangler r2 bucket delete <bucket>`. Missing resources are treated as
/// already-destroyed.
#[action]
#[derive(Default, Component)]
pub async fn CloudflareDestroyAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let destroy = cx.caller.get_cloned::<CloudflareDestroy>().await?;
	info!("deleting worker `{}`", destroy.name());
	ChildProcess::new("wrangler")
		.with_args(["delete", "--name", destroy.name().as_str(), "--force"])
		.run_async()
		.await
		.ok();
	// deleting the worker leaves the container application and its pushed
	// managed-registry image behind (they are not cascade-deleted), so remove both
	// explicitly or they keep billing.
	delete_container_app(destroy.name()).await;
	delete_container_images(destroy.name()).await;
	// empty the bucket first: `wrangler r2 bucket delete` refuses a non-empty
	// bucket, so delete the objects that were synced from `local_dir` (the same
	// keys `CloudflareR2Sync` uploaded). Missing objects are ignored.
	if let Some(local_dir) = destroy.local_dir() {
		empty_synced_objects(local_dir.as_str(), destroy.bucket()).await?;
	}
	info!("deleting r2 bucket `{}`", destroy.bucket());
	ChildProcess::new("wrangler")
		.with_args(["r2", "bucket", "delete", destroy.bucket().as_str()])
		.run_async()
		.await
		.ok();
	Pass(cx.input).xok()
}

/// Delete the container application Cloudflare created for `worker_name`. Lists
/// the apps as json, finds the one named `<worker>-<class>` (the only stable
/// handle, since `wrangler containers delete` takes the generated id, not the
/// name), and deletes it by id. A worker with no container is a no-op.
async fn delete_container_app(worker_name: &str) {
	let app_name = container_app_name(worker_name);
	let Ok(json) = ChildProcess::new("wrangler")
		.with_args(["containers", "list", "--json"])
		.run_async_stdout()
		.await
	else {
		return;
	};
	// `[{ id, name, ... }]`; match our app by name and delete by id. Explicit
	// loop rather than iterator closures: closures borrowing from `&Value` trip
	// a higher-ranked-lifetime inference bug once this future is boxed by `#[action]`.
	let mut id = None;
	if let Ok(serde_json::Value::Array(apps)) =
		serde_json::from_str::<serde_json::Value>(&json)
	{
		for app in &apps {
			if app["name"] == app_name.as_str() {
				id = app["id"].as_str().map(str::to_string);
				break;
			}
		}
	}
	if let Some(id) = id {
		info!("deleting container app `{app_name}` ({id})");
		ChildProcess::new("wrangler")
			.with_args(["containers", "delete", &id])
			.run_async()
			.await
			.ok();
	}
}

/// Delete every managed-registry image pushed for `worker_name`. Lists the repos
/// as json (`[{ name, tags }]`), then deletes each `<repo>:<tag>` whose repo is
/// the container app's. An empty registry is a no-op.
async fn delete_container_images(worker_name: &str) {
	let repo = container_app_name(worker_name);
	let Ok(json) = ChildProcess::new("wrangler")
		.with_args(["containers", "images", "list", "--json"])
		.run_async_stdout()
		.await
	else {
		return;
	};
	let Ok(serde_json::Value::Array(repos)) =
		serde_json::from_str::<serde_json::Value>(&json)
	else {
		return;
	};
	// Explicit loops over the json (see `delete_container_app`): iterator closures
	// borrowing from `&Value` break HRTB inference inside the boxed `#[action]` future.
	for entry in &repos {
		if entry["name"] != repo.as_str() {
			continue;
		}
		let Some(tags) = entry["tags"].as_array() else {
			continue;
		};
		for tag in tags {
			let Some(tag) = tag.as_str() else {
				continue;
			};
			let image = format!("{repo}:{tag}");
			info!("deleting container image `{image}`");
			ChildProcess::new("wrangler")
				.with_args(["containers", "images", "delete", &image])
				.run_async()
				.await
				.ok();
		}
	}
}

/// Delete every object in `bucket` whose key matches a file under `local_dir`,
/// the inverse of [`CloudflareR2SyncAction`]. Per-object failures are ignored so
/// an already-empty bucket (or a partial prior sync) still tears down.
async fn empty_synced_objects(local_dir: &str, bucket: &str) -> Result {
	// cwd-relative, matching `sync_dir_to_r2` (the same keys it uploaded).
	let root = AbsPathBuf::new(local_dir)?;
	let files = ReadDir::files_recursive(&root)?;
	info!("emptying {} synced objects from r2://{bucket}", files.len());
	for file in files {
		let rel = file.strip_prefix(&root).unwrap_or(file.as_path());
		let key = rel.to_string_lossy().replace('\\', "/");
		ChildProcess::new("wrangler")
			.with_args([
				"r2",
				"object",
				"delete",
				&format!("{bucket}/{key}"),
				"--remote",
			])
			.run_async()
			.await
			.ok();
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn fmt_bytes_scales_units() {
		fmt_bytes(512).as_str().xpect_eq("512.0 B");
		fmt_bytes(1024).as_str().xpect_eq("1.0 KB");
		fmt_bytes(1536).as_str().xpect_eq("1.5 KB");
		fmt_bytes(17_500_000).as_str().xpect_eq("16.7 MB");
	}
}
