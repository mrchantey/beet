//! Cloudflare deploy actions, driven by the `wrangler` CLI as `ChildProcess`
//! `#[action]`s (mirroring [`BuildDockerImage`]). Used by the
//! [`CloudflareContainerBlock`] and [`CloudflareWorkerBlock`] examples.
//!
//! `wrangler` is the deploy tool (not OpenTofu): it has first-class
//! `r2 bucket create/delete`, `deploy`, `delete` and `tail`, and natively builds
//! + pushes a container image (or runs `worker-build` for a wasm Worker) on
//! `deploy`. The `cf` CLI is a thinner JSON-over-REST wrapper and is the
//! documented fallback.
//!
//! Live deploy needs `CLOUDFLARE_API_TOKEN` in the environment and a
//! `{CloudflareAccount("..")}` on the stack or an ancestor, and nothing else:
//! the R2 data-plane pair the container reads the site with ([`S3Store::r2`])
//! and the teardown empties the bucket with is DERIVED from that same token
//! ([`CloudflareAccess::r2_credentials`]), never held as a second credential.
//! The Worker path needs no pair at all to deploy (native `worker::Bucket`
//! binding). Each action declares the groups its calls need on itself
//! ([`CloudflareAccess`]), which is also how it reaches the token. All commands
//! are `--dry-run`-able; see each example's module doc.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

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
async fn wrangler_r2_create(access: &CloudflareAccess, bucket: &str) -> Result {
	info!("ensuring R2 bucket `{bucket}`");
	// `run_async` errors on a non-zero exit, folding wrangler's stderr into the
	// error message, so match on that: `10004` / "already exists" / "already
	// owned" are the idempotent cases (the bucket is already there and ours).
	match access
		.wrangler()
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

/// The [`CloudflareAccount`] the action's entity resolves by ancestry, an
/// error naming the spread when none is declared.
async fn cloudflare_account(
	cx: &ActionContext<Request>,
) -> Result<CloudflareAccount> {
	cx.caller
		.with_state::<StackQuery, _>(|entity, stacks| {
			stacks.resolve(entity).cloudflare_account().cloned()
		})
		.await?
}

/// Find a sibling component of type `T` by walking the action's parent's children
/// (the same pattern [`BuildDockerImage`] uses for its block + artifact).
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
#[action]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
// `wrangler deploy` of the fronting Worker, the image it builds and pushes to
// the managed registry, and the bucket the container reads
#[require(CloudflareAccess = CloudflareAccess::new::<Self>(&[
	TokenPermission::ACCOUNT_SETTINGS_READ,
	TokenPermission::CLOUDCHAMBER_WRITE,
	TokenPermission::WORKERS_CONTAINERS_WRITE,
	TokenPermission::WORKERS_R2_STORAGE_WRITE,
	TokenPermission::WORKERS_SCRIPTS_WRITE,
]))]
pub async fn CloudflareContainerDeployAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let access = CloudflareAccess::resolve(&cx.caller).await?;
	let block = sibling::<CloudflareContainerBlock>(&cx).await?;
	let artifact = sibling::<BuildArtifact>(&cx).await?;

	let binary = AbsPath::new(artifact.artifact_path())?;
	if !fs_ext::exists(&binary)? {
		bevybail!("binary not found at: {}", binary);
	}

	// the R2 endpoint the container's `S3Store::r2` reads through, at the
	// account the stack declares; the account is also what addresses the
	// managed registry on deploy.
	let account = cloudflare_account(&cx).await?;
	let endpoint = account.r2_endpoint();

	let dir = wrangler_ext::project_dir(block.name())?;
	let binary_name = "beet";
	std::fs::copy(&binary, dir.join(binary_name))?;
	write_container_dockerfile(&dir, binary_name, &block, &endpoint)?;
	write_container_worker_js(&dir, &block)?;
	write_container_wrangler(&dir, &block)?;
	write_container_package_json(&dir)?;
	let secrets_file =
		write_r2_secrets_file(&access, &dir, account.id()).await?;

	// `wrangler deploy` bundles `worker.js`, whose `@cloudflare/containers` import
	// is resolved from `node_modules`, so install deps before deploying.
	npm_install(&dir).await?;
	wrangler_r2_create(&access, block.bucket()).await?;
	// upload the R2 keys as real Worker secrets with this version (`.dev.vars` is
	// otherwise local-only), so the container's `this.env.R2_*` reads resolve.
	wrangler_ext::deploy(&access, &dir, secrets_file.as_deref()).await?;
	info!("deployed container worker `{}`", block.name());
	Pass(cx.input).xok()
}

/// The Dockerfile: the native `beet` binary on debian-slim, serving http on the
/// container port. The site is pulled from R2 at boot, not baked in: the `CMD`
/// renders the block's argv-channel [`BootstrapConfig`] (the R2 store uri and the
/// `--server` selection, both known at deploy time), so deploy config reaches the
/// binary as argv; only the R2 credentials stay env (SDK convention, injected by
/// the fronting Worker).
fn write_container_dockerfile(
	dir: &AbsPath,
	binary_name: &str,
	block: &CloudflareContainerBlock,
	endpoint: &str,
) -> Result {
	let port = block.port();
	// the port is driven by the served site's markup `HttpServer{port}` (the
	// binary loads it from R2 at boot), so the container only needs to EXPOSE it.
	// A real JSON array, so the encoding is correct by construction.
	let cmd = block.cmd_bootstrap(endpoint).to_cmd_json("/app")?;
	let dockerfile = format!(
		"FROM debian:bookworm-slim\n\
		 RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*\n\
		 COPY {binary_name} /app\n\
		 RUN chmod +x /app\n\
		 EXPOSE {port}\n\
		 CMD {cmd}\n"
	);
	fs_ext::write(dir.join("Dockerfile"), dockerfile)?;
	Ok(())
}

/// The fronting Worker: a `Container` Durable Object that proxies every request
/// to the container. The entry-store selection is baked into the image `CMD` as
/// args ([`write_container_dockerfile`]); env carries only what genuinely is env:
/// the R2 credentials (the SDK convention, read from the Worker's secrets) and
/// the host bind.
fn write_container_worker_js(
	dir: &AbsPath,
	block: &CloudflareContainerBlock,
) -> Result {
	let port = block.port();
	let sleep_after = block.sleep_after();
	// non-secret env as literals, rendered from the block's env-channel
	// `BootstrapConfig` and JSON-escaped through serde rather than hand-written.
	// `BEET_HOST=0.0.0.0` binds the server to all interfaces (matching Fargate /
	// Lightsail): the fronting Worker proxies to the container's own IP, so a
	// default localhost bind would be unreachable ("not listening in the TCP
	// address <ip>:<port>").
	let mut env_lines = block
		.runtime_bootstrap()
		.to_env()
		.iter()
		.map(|(key, value)| -> Result<String> {
			format!("    {key}: {},\n", serde_json::to_string(value.as_str())?)
				.xok()
		})
		.collect::<Result<String>>()?;
	// secrets (the R2 keys) read from `this.env` at runtime, never rendered as a
	// literal here.
	env_lines.push_str(
		"    AWS_ACCESS_KEY_ID: this.env.R2_ACCESS_KEY_ID,\n\
		 \x20   AWS_SECRET_ACCESS_KEY: this.env.R2_SECRET_ACCESS_KEY,\n",
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
fn write_container_package_json(dir: &AbsPath) -> Result {
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
async fn npm_install(dir: &AbsPath) -> Result {
	info!("npm install ({})", dir);
	ChildProcess::new("npm")
		.with_args(["install", "--no-audit", "--no-fund", "--loglevel=error"])
		.with_cwd(dir.clone())
		.run_async()
		.await?;
	Ok(())
}

/// `wrangler.jsonc` binding the container Durable Object + the R2 bucket.
fn write_container_wrangler(
	dir: &AbsPath,
	block: &CloudflareContainerBlock,
) -> Result {
	let json = serde_json::to_string_pretty(&serde_json::json!({
		"name": block.name(),
		"main": "worker.js",
		"compatibility_date": wrangler_ext::COMPATIBILITY_DATE,
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

/// Write the R2 data-plane pair derived from the api token to a `.env`-format
/// secrets file (`secrets.env`) the deploy uploads as real Worker secrets
/// (`wrangler deploy --secrets-file`). Returns the file name (relative to the
/// project dir, which is the deploy cwd), or `None` with no token in the
/// environment, so a credential-free run still writes the project.
///
/// The container's env var names are the pair's, and what the Worker holds is
/// as wide as the token this deploy ran with: an example deployed by hand a
/// few times a year takes a wider credential for the command, and its bucket
/// holds nothing but the published site. A container of its own would take a
/// token scoped to one bucket's objects, the way an [`R2BucketBlock`]'s is.
async fn write_r2_secrets_file(
	access: &CloudflareAccess,
	dir: &AbsPath,
	account: &str,
) -> Result<Option<String>> {
	if access.token().is_err() {
		warn!(
			"CLOUDFLARE_API_TOKEN unset, so no R2 pair could be derived; the \
			 container cannot read R2 until one is uploaded as Worker secrets"
		);
		return None.xok();
	}
	let (access_key, secret_key) = access.r2_credentials(account).await?;
	let file_name = "secrets.env";
	fs_ext::write(
		dir.join(file_name),
		format!(
			"R2_ACCESS_KEY_ID={access_key}\nR2_SECRET_ACCESS_KEY={secret_key}\n"
		),
	)?;
	Some(file_name.to_string()).xok()
}

// ───────────────────────────── worker build ────────────────────────────────

/// Compile the wasm Worker and publish its artifacts to [`WORKER_ASSETS_DIR`]
/// without deploying. The slow step (a full wasm-bindgen + wasm-opt build) that
/// `deploy` and `bench` reuse; running it as its own verb makes the artifacts
/// (and the wasm size) visible before an upload, and warms the build so a
/// following `deploy` only uploads.
#[action]
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
	let cli_dir = AbsPath::new_workspace_rel("crates/beet-cli")?;
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
	let assets_dir = AbsPath::new_workspace_rel(WORKER_ASSETS_DIR)?;
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
	let index = AbsPath::new_workspace_rel(WORKER_ASSETS_DIR)?.join("index.js");
	if !fs_ext::exists(&index)? {
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
#[action]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
// the Worker upload, its secrets, the bucket it binds and the custom domain the
// upload provisions (with the record and certificate)
#[require(CloudflareAccess = CloudflareAccess::new::<Self>(&[
	TokenPermission::ACCOUNT_SETTINGS_READ,
	TokenPermission::WORKERS_R2_STORAGE_WRITE,
	TokenPermission::WORKERS_ROUTES_WRITE,
	TokenPermission::WORKERS_SCRIPTS_WRITE,
]))]
pub async fn CloudflareWorkerDeployAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let access = CloudflareAccess::resolve(&cx.caller).await?;
	let start = Instant::now();
	let block = sibling::<CloudflareWorkerBlock>(&cx).await?;
	ensure_worker_artifacts().await?;
	let dir = wrangler_ext::project_dir(block.name())?;
	write_worker_wrangler(&dir, &block, &worker_vars(&cx, &block).await?)?;

	wrangler_r2_create(&access, block.bucket()).await?;
	wrangler_ext::deploy(&access, &dir, None).await?;
	info!(
		"deployed wasm worker `{}` in {}",
		block.name(),
		time_ext::pretty_print_duration(start.elapsed()),
	);
	Pass(cx.input).xok()
}

/// The Worker's `vars`: the boot config the deploy bakes, ie the stack's repo
/// store ([`RepoStoreQuery::bootstrap`], the block itself) under the names the
/// runtime parses, exactly as the lambda and lightsail blocks bake theirs; then
/// the block's own, resolved against the request.
async fn worker_vars(
	cx: &ActionContext<Request>,
	block: &CloudflareWorkerBlock,
) -> Result<BTreeMap<SmolStr, SmolStr>> {
	let mut vars = cx
		.caller
		.with_state::<RepoStoreQuery, _>(|entity, repos| {
			repos.bootstrap(entity)
		})
		.await??
		.to_env()
		.into_iter()
		.collect::<BTreeMap<_, _>>();
	let parts = cx.input.parts();
	for var in block.env_vars() {
		vars.insert(var.key().clone(), var.resolve_value(parts)?);
	}
	vars.xok()
}

/// Write [`worker_wrangler_json`] into the wrangler project directory.
fn write_worker_wrangler(
	dir: &AbsPath,
	block: &CloudflareWorkerBlock,
	vars: &BTreeMap<SmolStr, SmolStr>,
) -> Result {
	// `main` is the prebuilt `index.js` (the wasm-bindgen entry; its `index_bg.wasm`
	// sibling resolves by relative import). An absolute path outside this wrangler
	// project dir is fine: wrangler bundles `main` and follows its wasm import.
	let main_js =
		AbsPath::new_workspace_rel(WORKER_ASSETS_DIR)?.join("index.js");
	fs_ext::write(
		dir.join("wrangler.jsonc"),
		worker_wrangler_json(block, &main_js.to_string(), vars)?,
	)?;
	Ok(())
}

/// `wrangler.jsonc` for the wasm Worker: `main` points at the prebuilt artifacts
/// (no `build.command`, so the deploy uploads them as-is), plus the R2 bucket
/// bound by the block's [`binding`](CloudflareWorkerBlock::binding), the
/// `vars` ([`worker_vars`]) and any custom domains the block declares.
fn worker_wrangler_json(
	block: &CloudflareWorkerBlock,
	main_js: &str,
	vars: &BTreeMap<SmolStr, SmolStr>,
) -> Result<String> {
	// `custom_domain` rather than a route pattern: wrangler then provisions the
	// zone record and the certificate, so a declared host is reachable over
	// https with nothing else to publish.
	let routes = block
		.routes()
		.iter()
		.map(|host| {
			serde_json::json!({
				"pattern": host,
				"custom_domain": true,
			})
		})
		.collect::<Vec<_>>();
	let mut config = serde_json::json!({
		"name": block.name(),
		"main": main_js,
		"compatibility_date": wrangler_ext::COMPATIBILITY_DATE,
		"compatibility_flags": ["nodejs_compat"],
		"r2_buckets": [{
			"binding": block.binding(),
			"bucket_name": block.bucket(),
		}],
		"vars": vars,
	});
	// omitted rather than empty: wrangler treats an empty `routes` as "serve
	// nowhere" and unpublishes the `workers.dev` host.
	if !routes.is_empty() {
		config["routes"] = routes.into();
	}
	serde_json::to_string_pretty(&config)?.xok()
}

// ───────────────────────────── R2 site sync ────────────────────────────────

/// Publishes a local site directory to an R2 bucket with one `aws s3 sync`
/// over R2's S3 endpoint, as the S3 pair the deploy token derives
/// ([`CloudflareAccess::r2_credentials`]), timing the publish (the headline
/// the `bench` verb measures against a full redeploy). Uploads what changed
/// and never deletes.
///
/// Over the S3 api rather than `wrangler r2 object put` because only S3
/// accepts the bucket-scoped object group, so the deploy token holds object
/// write on exactly this bucket and nothing account-wide: no other bucket's
/// objects, and no bucket's configuration (its lock among it).
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(CloudflareAccess = CloudflareAccess::new::<Self>(&[
	TokenPermission::BUCKET_ITEM_WRITE,
])
.with_bucket(|entity| entity.get::<Self>().map(|sync| sync.bucket.clone())))]
pub async fn CloudflareR2Sync(
	/// Local directory to publish (cwd-relative), eg `examples/bsx_site`.
	#[field]
	local_dir: SmolPath,
	/// Target R2 bucket.
	#[field]
	bucket: SmolStr,
	/// Optional R2 key prefix: each uploaded key becomes `<prefix>/<relpath>`
	/// instead of `<relpath>`, mounting a local directory under a sub-path of the
	/// bucket. Absent uploads to the bucket root.
	#[field]
	prefix: Option<RelPath>,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let access = CloudflareAccess::resolve(&cx.caller).await?;
	let account = cloudflare_account(&cx).await?;
	let start = Instant::now();
	sync_dir_to_r2(
		&access,
		&account,
		local_dir.as_str(),
		&bucket,
		prefix.as_ref().map(|prefix| prefix.as_str()),
	)
	.await?;
	info!(
		"synced site in {} (live on the next fetch)",
		time_ext::pretty_print_duration(start.elapsed()),
	);
	Pass(cx.input).xok()
}

impl CloudflareR2Sync {
	/// Publish `local_dir` to `bucket` at the bucket root.
	pub fn new(
		local_dir: impl Into<SmolPath>,
		bucket: impl Into<SmolStr>,
	) -> Self {
		Self {
			local_dir: local_dir.into(),
			bucket: bucket.into(),
			prefix: None,
		}
	}
}

/// Sync `local_dir` into `bucket` as the deploy token's derived S3 pair: one
/// process for the whole directory, uploading what changed and deleting
/// nothing, so a deploy only ever adds and replaces. Shared by
/// [`CloudflareR2Sync`] and [`CloudflareBench`].
///
/// `local_dir` is resolved relative to the cwd (like `--entry`), not the
/// workspace: the site is the user's, and a deploy `.bsx` may be run from a
/// different repo than the beet workspace that holds the Worker source.
///
/// `prefix`, when set, is prepended to every key (`<prefix>/<relpath>`), so a
/// directory can mount under a bucket sub-path (eg workspace `assets/` under
/// the site's `assets/` prefix). `None` syncs to the bucket root.
async fn sync_dir_to_r2(
	access: &CloudflareAccess,
	account: &CloudflareAccount,
	local_dir: &str,
	bucket: &str,
	prefix: Option<&str>,
) -> Result {
	let root = AbsPath::new(local_dir)?;
	let target = match prefix {
		Some(prefix) => format!("s3://{bucket}/{prefix}"),
		None => format!("s3://{bucket}"),
	};
	info!("syncing {root} to {target}");
	let (access_key, secret_key) = access.r2_credentials(account.id()).await?;
	aws_cli_ext::r2(&account.r2_endpoint(), &access_key, &secret_key, [
		"s3",
		"sync",
		&root.to_string(),
		&target,
		"--only-show-errors",
	])
	.run_async()
	.await?;
	Ok(())
}

// ───────────────────────────── bench ───────────────────────────────────────

/// Benchmarks the two ways to change a deployed Worker's behavior: an R2 `sync`
/// (publish the site, served on the next fetch via the Worker's per-request
/// version check) versus a full Worker rebuild + redeploy. Prints a side-by-side
/// comparison: the headline of the infra demo.
///
/// Times an R2 `sync` (and, with a `url`, how soon the live Worker serves it)
/// against a full rebuild + redeploy, then prints the comparison.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
// times a redeploy against an R2 sync, so it asks for both paths
#[require(CloudflareAccess = CloudflareAccess::new::<Self>(&[
	TokenPermission::ACCOUNT_SETTINGS_READ,
	TokenPermission::WORKERS_R2_STORAGE_WRITE,
	TokenPermission::WORKERS_ROUTES_WRITE,
	TokenPermission::WORKERS_SCRIPTS_WRITE,
]))]
pub async fn CloudflareBench(
	/// Worker name, redeployed to time the full-redeploy path.
	#[field]
	name: SmolStr,
	/// R2 bucket the site is published to.
	#[field]
	bucket: SmolStr,
	/// Local site directory published on the sync path (cwd-relative).
	#[field]
	local_dir: SmolPath,
	/// Optional live Worker URL; when set, the sync path also polls it until it
	/// serves a 200, timing how soon the fresh site is live.
	#[field]
	url: Option<SmolStr>,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let access = CloudflareAccess::resolve(&cx.caller).await?;
	let account = cloudflare_account(&cx).await?;
	// sync path: publish the site to R2; the Worker serves it on the next fetch.
	let sync_start = Instant::now();
	sync_dir_to_r2(&access, &account, local_dir.as_str(), &bucket, None)
		.await?;
	let sync_elapsed = sync_start.elapsed();

	// with a url, also time how soon the live Worker serves the fresh site.
	let live_elapsed = match &url {
		Some(url) => Some(poll_until_ok(url.as_str(), sync_start).await?),
		None => None,
	};

	// redeploy path: a full wasm rebuild + Worker redeploy (the prebuilt artifacts
	// are rebuilt, then uploaded).
	let redeploy_start = Instant::now();
	build_worker_artifacts().await?;
	let block =
		CloudflareWorkerBlock::new(name.clone()).with_bucket(bucket.clone());
	let dir = wrangler_ext::project_dir(&name)?;
	write_worker_wrangler(&dir, &block, &worker_vars(&cx, &block).await?)?;
	wrangler_ext::deploy(&access, &dir, None).await?;
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

/// Poll `url` until it serves a 200, returning how long after `since` that took.
/// Bounded so an unreachable Worker fails the bench instead of hanging.
///
/// Open-coded rather than through `poll_ext`: [`CloudflareBench`] awaits this
/// from an `#[action]`, whose future must be `Send`, and an `async ||` probe
/// borrowing `url` makes that obligation higher-ranked (rustc's
/// "higher-ranked lifetime error").
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
///
/// Tails a deployed Worker's logs via `wrangler tail`, the Cloudflare analogue
/// of [`AwsWatch`]. With a timeout it tails then stops; otherwise it follows.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
// `wrangler tail`, whose session is opened under the script's own group; a 403
// here is Cloudflare asking for `Workers Tail Read` instead, which no token of
// this account has ever held
#[require(CloudflareAccess = CloudflareAccess::new::<Self>(&[
	TokenPermission::ACCOUNT_SETTINGS_READ,
	TokenPermission::WORKERS_SCRIPTS_WRITE,
]))]
pub async fn CloudflareWatch(
	/// Worker name, used to list deployments and (optionally) poll the host.
	#[field]
	name: SmolStr,
	/// Optional poll timeout.
	#[field]
	timeout: Option<Duration>,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let access = CloudflareAccess::resolve(&cx.caller).await?;
	info!("tailing worker `{name}`");
	// group: `wrangler` is a wrapper that spawns the real node process; a plain
	// kill orphans it and the leaked tail holds stdio open past process exit.
	let mut child = access
		.wrangler()
		.with_args(["tail", name.as_str(), "--format", "pretty"])
		.with_group(true)
		.spawn()?;
	if let Some(timeout) = timeout {
		time_ext::sleep(timeout).await;
		child.kill().ok();
	} else {
		child.status().await?;
	}
	Pass(cx.input).xok()
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

/// Tears down a Cloudflare deploy: deletes the Worker (and any container app +
/// image), then empties and deletes its R2 bucket. Reads the sibling block
/// (container or worker) for the names; mandatory for the teardown gate.
///
/// `wrangler delete <worker>`, empty the bucket (deleting *every* object), then
/// `wrangler r2 bucket delete <bucket>`. Missing resources are treated as
/// already-destroyed.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
// deletes the script, its custom domain, the container application, its
// registry image and the bucket
#[require(CloudflareAccess = CloudflareAccess::new::<Self>(&[
	TokenPermission::ACCOUNT_SETTINGS_READ,
	TokenPermission::CLOUDCHAMBER_WRITE,
	TokenPermission::WORKERS_CONTAINERS_WRITE,
	TokenPermission::WORKERS_R2_STORAGE_WRITE,
	TokenPermission::WORKERS_ROUTES_WRITE,
	TokenPermission::WORKERS_SCRIPTS_WRITE,
]))]
pub async fn CloudflareDestroy(
	/// Worker name to delete.
	#[field]
	name: SmolStr,
	/// R2 bucket to empty + delete. Every object is removed regardless of prefix, so
	/// no local-directory hint is needed to find the synced keys.
	#[field]
	bucket: SmolStr,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let access = CloudflareAccess::resolve(&cx.caller).await?;
	info!("deleting worker `{name}`");
	access
		.wrangler()
		.with_args(["delete", "--name", name.as_str(), "--force"])
		.run_async()
		.await
		.ok();
	// deleting the worker leaves the container application and its pushed
	// managed-registry image behind (they are not cascade-deleted), so remove both
	// explicitly or they keep billing.
	delete_container_app(&access, &name).await;
	delete_container_images(&access, &name).await;
	// empty the bucket first: `wrangler r2 bucket delete` refuses a non-empty bucket
	// and `wrangler r2 object` cannot list, so clear *every* object (any prefix,
	// eg `site/*` and `assets/*`) through the R2 S3 endpoint before deleting.
	empty_bucket(&access, &cloudflare_account(&cx).await?, &bucket).await?;
	info!("deleting r2 bucket `{bucket}`");
	access
		.wrangler()
		.with_args(["r2", "bucket", "delete", bucket.as_str()])
		.run_async()
		.await
		.ok();
	Pass(cx.input).xok()
}

impl CloudflareDestroy {
	/// Destroy the named Worker + bucket.
	pub fn new(name: impl Into<SmolStr>, bucket: impl Into<SmolStr>) -> Self {
		Self {
			name: name.into(),
			bucket: bucket.into(),
		}
	}
}

/// Delete the container application Cloudflare created for `worker_name`. Lists
/// the apps as json, finds the one named `<worker>-<class>` (the only stable
/// handle, since `wrangler containers delete` takes the generated id, not the
/// name), and deletes it by id. A worker with no container is a no-op.
async fn delete_container_app(access: &CloudflareAccess, worker_name: &str) {
	let app_name = container_app_name(worker_name);
	let Ok(json) = access
		.wrangler()
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
		access
			.wrangler()
			.with_args(["containers", "delete", &id])
			.run_async()
			.await
			.ok();
	}
}

/// Delete every managed-registry image pushed for `worker_name`. Lists the repos
/// as json (`[{ name, tags }]`), then deletes each `<repo>:<tag>` whose repo is
/// the container app's. An empty registry is a no-op.
async fn delete_container_images(access: &CloudflareAccess, worker_name: &str) {
	let repo = container_app_name(worker_name);
	let Ok(json) = access
		.wrangler()
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
			access
				.wrangler()
				.with_args(["containers", "images", "delete", &image])
				.run_async()
				.await
				.ok();
		}
	}
}

/// Empty `bucket` completely — delete *every* object regardless of prefix — through
/// the R2 S3-compatible endpoint of `account`. `wrangler r2 object` cannot list
/// objects, so it cannot find keys synced under a prefix (eg the `assets/*`
/// mount); `aws s3 rm --recursive` lists + deletes them all, which `wrangler r2
/// bucket delete` then requires (it refuses a non-empty bucket). The S3 pair is
/// derived from the api token the teardown already runs with; with no token the
/// empty is skipped with a warning so a no-creds teardown still deletes the
/// worker.
async fn empty_bucket(
	access: &CloudflareAccess,
	account: &CloudflareAccount,
	bucket: &str,
) -> Result {
	if access.token().is_err() {
		warn!(
			"CLOUDFLARE_API_TOKEN unset, so no R2 pair could be derived; \
			 skipping the R2 empty (bucket delete fails if non-empty)"
		);
		return Ok(());
	}
	let (access_key, secret_key) = access.r2_credentials(account.id()).await?;
	let endpoint = account.r2_endpoint();
	info!("emptying all objects from r2://{bucket} via {endpoint}");
	match aws_cli_ext::r2(&endpoint, &access_key, &secret_key, [
		"s3",
		"rm",
		&format!("s3://{bucket}"),
		"--recursive",
	])
	.run_async()
	.await
	{
		Ok(_) => Ok(()),
		// an already-deleted bucket has nothing to empty; treat as done so a repeat
		// teardown is idempotent (mirrors `wrangler_r2_create`'s "already exists").
		Err(err)
			if err.to_string().contains("NoSuchBucket")
				|| err.to_string().contains("does not exist") =>
		{
			info!("r2://{bucket} already gone, nothing to empty");
			Ok(())
		}
		Err(err) => Err(err),
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// The R2 sync asks for object write and names its own bucket for it, so
	/// the deploy token is granted that bucket's objects and nothing
	/// account-wide.
	#[beet_core::test]
	fn the_sync_names_its_bucket() {
		let mut world = World::new();
		let sync = world
			.spawn(CloudflareR2Sync::new("examples/bsx_site", "site"))
			.id();
		let access = CloudflareAccess::of(world.entity(sync)).unwrap();
		access
			.permissions()
			.xpect_eq(&[TokenPermission::BUCKET_ITEM_WRITE][..]);
		access
			.bucket(world.entity(sync))
			.xpect_eq(Some(SmolStr::from("site")));
	}

	/// A declared host lands as a wrangler custom domain, so wrangler creates
	/// its record and certificate. Without one the key is absent entirely: an
	/// empty `routes` array means "serve nowhere" and takes the `workers.dev`
	/// host down with it.
	#[beet_core::test]
	fn routes_render_as_custom_domains() {
		let block = CloudflareWorkerBlock::new("mta-sts");
		worker_wrangler_json(&block, "index.js", &default())
			.unwrap()
			.as_str()
			.xnot()
			.xpect_contains("routes");
		worker_wrangler_json(
			&block.with_route("mta-sts.stalwart.beetmash.com"),
			"index.js",
			&default(),
		)
		.unwrap()
		.as_str()
		.xpect_contains("\"pattern\": \"mta-sts.stalwart.beetmash.com\"")
		.xpect_contains("\"custom_domain\": true");
	}

	/// The bucket is bound under the block's binding, and the `vars` carry the
	/// repo store as `BEET_REPO=r2://<binding>`, so the Worker boots from the
	/// same binding wrangler bound.
	#[beet_core::test]
	fn binds_the_bucket_and_bakes_the_repo_store() {
		let block = CloudflareWorkerBlock::new("hello")
			.with_bucket("hello-site")
			.with_binding("SITE");
		let vars = BootstrapConfig {
			repo: Some(
				block
					.store_uri(&Stack::default().resolve(&default()))
					.unwrap(),
			),
			..default()
		}
		.to_env()
		.into_iter()
		.collect();
		worker_wrangler_json(&block, "index.js", &vars)
			.unwrap()
			.as_str()
			.xpect_contains("\"binding\": \"SITE\"")
			.xpect_contains("\"bucket_name\": \"hello-site\"")
			.xpect_contains("\"BEET_REPO\": \"r2://SITE\"");
	}

	#[beet_core::test]
	fn fmt_bytes_scales_units() {
		fmt_bytes(512).as_str().xpect_eq("512.0 B");
		fmt_bytes(1024).as_str().xpect_eq("1.0 KB");
		fmt_bytes(1536).as_str().xpect_eq("1.5 KB");
		fmt_bytes(17_500_000).as_str().xpect_eq("16.7 MB");
	}
}
