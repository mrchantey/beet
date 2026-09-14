//! [`AsyncTask`]: the handle to a spawned task, cancelling it on drop.
use bevy::platform::sync::Arc;
use bevy::platform::sync::Mutex;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use core::task::Context;
use core::task::Poll;
use core::task::Waker;

/// A spawned task. Dropping the handle cancels the task at its next poll;
/// [`detach`](Self::detach) lets it run to completion unowned.
///
/// Cancellation is beet's own, applied by
/// [`AsyncSpawner`](crate::prelude::AsyncSpawner) before the future reaches
/// whichever runtime runs it, so a backend without cancellation (tokio detaches
/// on drop, embassy has none) behaves exactly as the default executors do. A
/// cancelled future is dropped at its next poll without resuming, so whatever
/// its locals hold (a listener, a socket) closes with it and nothing after
/// the await it was parked on runs.
///
/// The `run_async` verbs never hand one out: a world task is detached, an
/// entity task is owned by its entity. The `run_task` twins (`queue_task` from
/// commands) return it for the caller to own, and the component naming the
/// work is where to keep it: removal, replacement by a later insert and the
/// entity's despawn all cancel through ordinary component drop, with no hook.
///
/// ```
/// # use beet_core::prelude::*;
/// /// A reload's async tail, cancelled by the next reload's insert.
/// #[derive(Component)]
/// struct ReloadTail(AsyncTask);
/// ```
#[must_use = "dropping the handle cancels the task, `detach` it to let it run"]
#[derive(Debug)]
pub struct AsyncTask(Arc<AsyncTaskInner>);

/// The state a handle and its task share.
#[derive(Debug, Default)]
struct AsyncTaskInner {
	/// The handle was dropped undetached.
	cancelled: AtomicBool,
	/// The task ended, by completing, cancelling or panicking.
	finished: AtomicBool,
	/// The handle let go, so its drop is not a cancel.
	detached: AtomicBool,
	/// The task's waker, so a cancel is observed at once rather than at the
	/// next unrelated wake.
	waker: Mutex<Option<Waker>>,
}

impl AsyncTask {
	/// A handle whose task is yet to be wrapped, see [`wrap`](Self::wrap).
	pub(crate) fn new() -> Self { Self(Arc::default()) }

	/// Wrap `fut` so it ends at the poll after this handle is dropped, marking
	/// the task finished however it ends.
	pub(crate) fn wrap<Fut>(
		&self,
		fut: Fut,
	) -> impl Future<Output = ()> + use<Fut>
	where
		Fut: Future<Output = ()>,
	{
		let inner = self.0.clone();
		async move {
			let _finished = Finished(inner.clone());
			// cancellation is checked first, so a cancelled task never resumes: its
			// future is dropped unpolled, as a runtime's own cancel would, and code
			// after an await it was parked on never runs
			futures_lite::future::or(Cancelled(inner), fut).await;
		}
	}

	/// Let the task run to completion, unowned.
	pub fn detach(self) { self.0.detached.store(true, Ordering::SeqCst); }

	/// Cancel the task, exactly as dropping the handle does.
	pub fn cancel(self) {}

	/// Whether the task has ended, by completing, cancelling or panicking.
	pub fn is_finished(&self) -> bool { self.0.finished.load(Ordering::SeqCst) }
}

impl Drop for AsyncTask {
	fn drop(&mut self) {
		if self.0.detached.load(Ordering::SeqCst) {
			return;
		}
		self.0.cancelled.store(true, Ordering::SeqCst);
		if let Some(waker) =
			self.0.waker.lock().ok().and_then(|mut waker| waker.take())
		{
			waker.wake();
		}
	}
}

/// Resolves once the handle is dropped undetached.
struct Cancelled(Arc<AsyncTaskInner>);

impl Future for Cancelled {
	type Output = ();
	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
		// the waker lands before the flag is read, so a cancel between the two
		// still wakes this poll's successor
		if let Ok(mut waker) = self.0.waker.lock()
			&& !waker
				.as_ref()
				.is_some_and(|waker| waker.will_wake(cx.waker()))
		{
			*waker = Some(cx.waker().clone());
		}
		match self.0.cancelled.load(Ordering::SeqCst) {
			true => Poll::Ready(()),
			false => Poll::Pending,
		}
	}
}

/// Marks the task finished when dropped, so completion, cancellation and a
/// panic all report the same way.
struct Finished(Arc<AsyncTaskInner>);

impl Drop for Finished {
	fn drop(&mut self) { self.0.finished.store(true, Ordering::SeqCst); }
}

#[cfg(all(test, feature = "std"))]
mod test {
	use crate::prelude::*;

	fn app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin));
		app
	}

	/// Spawn a task that sets `reached` after a short sleep, returning its handle.
	fn sleep_then_set(app: &mut App, reached: &Store<bool>) -> AsyncTask {
		let reached = reached.clone();
		app.world_mut().run_task_local(move |_| async move {
			time_ext::sleep_millis(20).await;
			reached.set(true);
		})
	}

	#[crate::test]
	async fn drop_cancels() {
		let mut app = app();
		let reached = Store::<bool>::default();
		let task = sleep_then_set(&mut app, &reached);
		task.cancel();
		app.update_async().await;
		reached.get().xpect_false();
		app.world()
			.resource::<AsyncSpawner>()
			.in_flight()
			.xpect_eq(0);
	}

	#[crate::test]
	async fn detach_runs_to_completion() {
		let mut app = app();
		let reached = Store::<bool>::default();
		sleep_then_set(&mut app, &reached).detach();
		app.update_async().await;
		reached.get().xpect_true();
	}

	#[crate::test]
	async fn finishes() {
		let mut app = app();
		let reached = Store::<bool>::default();
		let task = sleep_then_set(&mut app, &reached);
		task.is_finished().xpect_false();
		app.update_async().await;
		task.is_finished().xpect_true();
		reached.get().xpect_true();
	}

	/// A task parked on a bridge request is cancelled between the request being
	/// queued and the sync point that would serve it: the sync point must then
	/// neither hang nor panic on the request's dropped future.
	#[crate::test]
	async fn cancel_while_parked_on_the_bridge() {
		let mut app = app();
		let reached = Store::<bool>::default();
		let task = {
			let reached = reached.clone();
			app.world_mut().run_task_local(move |world| async move {
				world.with(|_| ()).await;
				reached.set(true);
			})
		};
		// poll once so the request is queued, then cancel and poll again outside
		// the sync point so the cancel is what the task observes
		AsyncRunner::tick(app.world()).await;
		drop(task);
		AsyncRunner::tick(app.world()).await;
		app.update_async().await;
		reached.get().xpect_false();
		app.world()
			.resource::<AsyncSpawner>()
			.in_flight()
			.xpect_eq(0);
	}

	/// The task lives in a component: despawning cancels it.
	#[derive(Component)]
	struct Owner(#[allow(dead_code)] AsyncTask);

	#[crate::test]
	async fn despawn_cancels() {
		let mut app = app();
		let reached = Store::<bool>::default();
		let task = sleep_then_set(&mut app, &reached);
		app.world_mut().spawn(Owner(task)).despawn();
		app.update_async().await;
		reached.get().xpect_false();
	}

	#[crate::test]
	async fn replacement_cancels_the_predecessor() {
		let mut app = app();
		let (first, second) =
			(Store::<bool>::default(), Store::<bool>::default());
		let task = sleep_then_set(&mut app, &first);
		let entity = app.world_mut().spawn(Owner(task)).id();
		let task = sleep_then_set(&mut app, &second);
		app.world_mut().entity_mut(entity).insert(Owner(task));
		app.update_async().await;
		first.get().xpect_false();
		second.get().xpect_true();
	}
}
