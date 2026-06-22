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
async fn sibling<T: Component + Clone>(cx: &ActionContext<Request>) -> Result<T> {
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
#[action]
#[derive(Default, Component)]
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

// ───────────────────────────── worker deploy ───────────────────────────────

/// Deploy `beet-cli` (wasm) as a Cloudflare Worker: write `wrangler.jsonc` with
/// the R2 binding and a `worker-build` build command, ensure the R2 bucket, then
/// `wrangler deploy` (which runs `worker-build` and uploads the wasm). Reads the
/// sibling [`CloudflareWorkerBlock`].
///
/// The wasm artifact is produced from the `beet-cli` crate (`--features
/// cloudflare`) by `worker-build`, invoked by wrangler's `build.command`, so this
/// action carries no separate [`BuildArtifact`].
#[action]
#[derive(Default, Component)]
pub async fn CloudflareWorkerDeployAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let block = sibling::<CloudflareWorkerBlock>(&cx).await?;
	let dir = cf_project_dir(block.name())?;
	write_worker_wrangler(&dir, &block)?;

	wrangler_r2_create(block.bucket()).await?;
	wrangler_deploy(&dir, None).await?;
	info!("deployed wasm worker `{}`", block.name());
	Pass(cx.input).xok()
}

/// `wrangler.jsonc` for the wasm Worker: a `worker-build` build command, the wasm
/// main, and the R2 bucket bound by [`WORKER_R2_BINDING`].
fn write_worker_wrangler(
	dir: &AbsPathBuf,
	block: &CloudflareWorkerBlock,
) -> Result {
	let cli_dir = AbsPathBuf::new_workspace_rel("crates/beet-cli")?;
	// worker-build always writes its output relative to the *crate root*, not the
	// process cwd (it emits `<crate>/build/worker/shim.mjs`), so `main` points at
	// that absolute path rather than a path relative to this wrangler project dir.
	let shim = cli_dir.join("build").join("worker").join("shim.mjs");
	let vars = block
		.env_vars()
		.iter()
		.map(|var| (var.key().to_string(), var.key().to_string()))
		.collect::<std::collections::BTreeMap<_, _>>();
	let json = serde_json::to_string_pretty(&serde_json::json!({
		"name": block.name(),
		"main": shim.to_string(),
		"compatibility_date": COMPATIBILITY_DATE,
		"compatibility_flags": ["nodejs_compat"],
		// wrangler runs this before uploading; worker-build wraps wasm-bindgen +
		// wasm-opt and emits `<beet-cli>/build/worker/shim.mjs` + the wasm, which
		// `main` above points at.
		"build": {
			"command": format!(
				"worker-build --release {} -- --no-default-features --features cloudflare",
				cli_dir.display()
			),
		},
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
#[derive(Debug, Clone, Get, SetWith, Component)]
#[require(CloudflareR2SyncAction)]
pub struct CloudflareR2Sync {
	/// Local directory to publish (workspace-relative), eg `examples/bsx_site`.
	local_dir: SmolPath,
	/// Target R2 bucket.
	bucket: SmolStr,
}

impl CloudflareR2Sync {
	/// Publish `local_dir` to `bucket`.
	pub fn new(local_dir: impl Into<SmolPath>, bucket: impl Into<SmolStr>) -> Self {
		Self {
			local_dir: local_dir.into(),
			bucket: bucket.into(),
		}
	}
}

/// Walk the [`CloudflareR2Sync`] directory and upload each file to R2 via
/// `wrangler r2 object put <bucket>/<key> --file <path>`.
#[action]
#[derive(Default, Component)]
pub async fn CloudflareR2SyncAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let sync = cx.caller.get_cloned::<CloudflareR2Sync>().await?;
	let root = AbsPathBuf::new_workspace_rel(sync.local_dir().as_str())?;
	let files = ReadDir::files_recursive(&root)?;
	info!(
		"syncing {} files from {} to r2://{}",
		files.len(),
		root.display(),
		sync.bucket()
	);
	for file in files {
		let rel = file.strip_prefix(&root).unwrap_or(file.as_path());
		let key = rel.to_string_lossy().replace('\\', "/");
		let file_arg = file.to_string_lossy().to_string();
		// `--remote` targets the real R2 bucket; without it wrangler writes to its
		// *local* Miniflare store, which a deployed Worker never reads.
		ChildProcess::new("wrangler")
			.with_args([
				"r2",
				"object",
				"put",
				&format!("{}/{key}", sync.bucket()),
				"--file",
				&file_arg,
				"--remote",
			])
			.run_async()
			.await?;
	}
	Pass(cx.input).xok()
}

// ───────────────────────────── watch + destroy ─────────────────────────────

/// Polls a deployed Worker for readiness (the deploy + rollout is near-instant on
/// Cloudflare, unlike an ECS rollout). Reads the host (`<name>.workers.dev`) it
/// was constructed with.
#[derive(Debug, Clone, Get, SetWith, Component)]
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
	let mut child = ChildProcess::new("wrangler")
		.with_args(["tail", watch.name().as_str(), "--format", "pretty"])
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
#[derive(Debug, Clone, Get, SetWith, Component)]
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
	let root = AbsPathBuf::new_workspace_rel(local_dir)?;
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
