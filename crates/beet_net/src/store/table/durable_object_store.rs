use crate::prelude::*;
use beet_core::prelude::*;

/// A table store whose every partition is a Cloudflare Durable Object: one
/// object per partition, named by `id_from_name(<partition>)` (`@` for the
/// declaration's own), owning that partition's SQLite storage and answering
/// the [`TableWire`] protocol over it. The Worker routes and does stateless
/// work; anything that reads then writes a partition runs in its object.
///
/// A declaration of several backends, resolved once when it is inserted
/// ([`client`](Self::client)) into the [`TableStore`] landing beside it, so
/// nothing forwards one method at a time:
/// - in a Worker (`wasm32` with `cloudflare`), a [`TableWireClient`] sending
///   each request to its partition's object through the namespace `binding`;
/// - natively, with local service access, the same client over an
///   in-process [`PartitionHost`] keeping each partition as a SQLite file
///   under `dir` (by default the workspace's `target/stores/<binding>`), its
///   requests to one partition answered one at a time as an object answers
///   its own: the objects' stand-in, as a local store is a Dynamo table's.
///
/// Its fallback differs from that of a bucket or a Dynamo table, which any
/// process can reach and so is a block, the launch's [`ServiceAccess`] picking
/// the remote or its local form. A Durable Object's storage is reached only
/// from a Worker, so a native build plainly is not using one: there it falls
/// back to SQLite files, remote access being refused.
///
/// So a runtime moves between a native host and Cloudflare by where it runs,
/// and nothing that reads the store knows which. The Worker's deploy renders
/// the binding and its `new_sqlite_classes` migration for the one object
/// class beet exports, [`DurableObjectStore::CLASS`].
///
/// ```html
/// <DurableObjectStore bx:ref="tables" binding="TABLES"/>
/// ```
///
/// Rows alone: bytes belong in a bucket (`R2WorkersStore`), a Durable Object's
/// row being at most 2MB.
#[derive(Debug, Clone, Get, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_insert = DurableObjectStore::on_insert)]
pub struct DurableObjectStore {
	/// The Durable Object namespace binding, ie `TABLES` in `env.TABLES`.
	binding: SmolStr,
	/// Where the native host keeps its databases, by default the
	/// workspace's `target/stores/<binding>`.
	#[reflect(default)]
	dir: Option<AbsPath>,
}

impl Default for DurableObjectStore {
	fn default() -> Self { Self::new("TABLES") }
}

impl DurableObjectStore {
	/// The object class every partition is an instance of, the one a Worker
	/// built from beet exports.
	pub const CLASS: &'static str = "BeetTableObject";
	/// The object name of the declaration's own partition: never a partition
	/// id, whose alphabet has no `@`.
	pub const ROOT: &'static str = "@";
	/// What every object supports, the same as a SQLite file.
	pub(crate) const CAPABILITIES: [Capability; 2] =
		[Capability::AtomicApply, Capability::Indexes];

	/// The store of the namespace `binding`.
	pub fn new(binding: impl Into<SmolStr>) -> Self {
		Self {
			binding: binding.into(),
			dir: None,
		}
	}

	/// Keep the native host's databases under `dir`.
	pub fn with_dir(mut self, dir: impl Into<AbsPath>) -> Self {
		self.dir = Some(dir.into());
		self
	}

	/// The client of this store where the process runs, resolved once: in a
	/// Worker the object client; natively by the launch's [`ServiceAccess`],
	/// local the [`PartitionHost`] and remote refused, a deployed Worker's
	/// tables being reachable from outside only once a request can be
	/// authenticated. A build with neither backend refuses every request, so
	/// a document declaring the store loads in any build, a deploy's included.
	pub fn client(&self) -> TableWireClient {
		cfg_if! {
			if #[cfg(all(target_arch = "wasm32", feature = "cloudflare"))] {
				Self::object_client(self.binding.clone())
			} else if #[cfg(feature = "sqlite")] {
				match BootstrapConfig::get().service_access {
					ServiceAccess::Local => {
						let dir = self.dir.clone().unwrap_or_else(|| {
							WsPath::new(format!("target/stores/{}", self.binding)).into()
						});
						TableWireClient::new(PartitionHost::new(dir).transport())
							.with_capabilities(Self::CAPABILITIES.to_vec())
					}
					ServiceAccess::Remote => self.refusing(
						"is reachable from outside its Worker only through an \
						 authenticated mount, which beet does not serve yet",
					),
				}
			} else {
				self.refusing(
					"runs as Durable Objects in a Worker (a wasm build with \
					 `cloudflare`) or as SQLite files natively (the `sqlite` \
					 feature), and this build has neither",
				)
			}
		}
	}

	/// A client refusing every request, saying why.
	#[allow(
		dead_code,
		reason = "every build but a Worker's resolves through it"
	)]
	fn refusing(&self, why: &str) -> TableWireClient {
		let message = format!("`DurableObjectStore` `{}` {why}", self.binding);
		TableWireClient::new(TableWireTransport::new(
			format!("durable-object:{}", self.binding),
			move |_request| {
				let message = message.clone();
				Box::pin(async move { Err(bevyhow!("{message}")) })
					as SendBoxedFuture<Result<Response>>
			},
		))
		.with_capabilities(Self::CAPABILITIES.to_vec())
	}

	/// Land the [`TableStore`] over the client.
	fn on_insert(mut world: DeferredWorld, cx: HookContext) {
		if let Some(store) = world.entity(cx.entity).get::<Self>().cloned() {
			world.commands().entity(cx.entity).queue_silenced(
				move |mut entity: EntityWorldMut| {
					TableStore::new(store.client()).land(&mut entity)
				},
			);
		}
	}
}

#[cfg(all(test, feature = "sqlite", not(target_arch = "wasm32")))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Natively the declaration is a partition host: its table store lands on
	/// the entity and no blob store does, and each partition is a file under
	/// its directory.
	#[beet_core::test]
	async fn resolves_to_a_partition_host() {
		let dir = TempDir::new_ws().unwrap();
		let mut world = World::new();
		let entity = world
			.spawn(
				DurableObjectStore::new("TABLES").with_dir(dir.path().clone()),
			)
			.id();
		world.flush();
		let store = world.entity(entity).get::<TableStore>().unwrap().clone();
		world.entity(entity).contains::<BlobStore>().xpect_false();
		store.supports(Capability::AtomicApply).xpect_true();
		store
			.partition("lobby")
			.unwrap()
			.table::<table_test::NamedRow>()
			.put(table_test::NamedRow::new("k", 1))
			.await
			.unwrap();
		fs_ext::exists(dir.path().join("root/lobby.db"))
			.unwrap()
			.xpect_true();
	}
}
