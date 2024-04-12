use futures::executor::LocalPool;
use futures::task::LocalSpawnExt;
use futures::task::SpawnError;
use futures::Future;


// copied from https://github.com/esp-rs/esp-idf-svc/blob/215363ff725b6a3fd1de205b7a33aa06e723546d/examples/tcp_async.rs#L50
pub fn spawn_local(
	fut: impl Future<Output = ()> + 'static,
) -> Result<(), SpawnError> {
	let mut local_executor = LocalPool::new();
	let spawner = local_executor.spawner();
	local_executor.spawner().spawn_local(fut)
}
