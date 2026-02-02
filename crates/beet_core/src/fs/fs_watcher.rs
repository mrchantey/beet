//! File system watcher with glob filtering.
//!
//! This module provides [`FsWatcher`], a component for watching directories for
//! file system changes. It uses debouncing to coalesce rapid changes and supports
//! glob-based filtering to include or exclude specific paths.
//!
//! # Usage
//!
//! Add the [`FsWatcher`] component to an entity to start watching a directory.
//! File system events will be delivered as [`DirEvent`] triggers on that entity.
//!
//! # Common Pitfalls
//!
//! - If the directory does not exist when the watcher starts, it will error
//! - If the watched path is removed while watching, the watcher will silently stop

use crate::prelude::*;
pub use async_channel::Receiver;
pub use async_channel::Sender;
use notify::EventKind;
use notify::RecursiveMode;
use notify::event::CreateKind;
use notify::event::RemoveKind;
use notify_debouncer_full::DebouncedEvent;
use notify_debouncer_full::new_debouncer;
use std::num::ParseIntError;
use std::time::Duration;

/// A file watcher with glob patterns.
///
/// All matches against `include` and `exclude` patterns will be normalized
/// to forward slashes.
///
/// # Common Pitfalls
///
/// - If the directory does not exist when the watcher starts, it will error
/// - If the [`Self::path`] is removed while watching, the watcher will silently stop listening
#[derive(Debug, Clone, Component)]
#[component(on_add=start_fs_watcher)]
pub struct FsWatcher {
	/// The path to watch.
	pub path: AbsPathBuf,
	/// Glob filter for paths to include/exclude.
	pub filter: GlobFilter,
	/// Debounce time in milliseconds.
	pub debounce: Duration,
	/// Only send events that mutated paths.
	pub mutated_only: bool,
}
impl Default for FsWatcher {
	fn default() -> Self {
		Self {
			path: AbsPathBuf::default(),
			filter: GlobFilter::default(),
			debounce: Duration::from_millis(50),
			mutated_only: true,
		}
	}
}


/// Parses a duration from a string of milliseconds.
pub fn parse_duration(s: &str) -> Result<Duration, ParseIntError> {
	s.parse().map(Duration::from_millis)
}

impl FsWatcher {
	/// Creates a new [`FsWatcher`] for the given path.
	pub fn new(path: AbsPathBuf) -> Self { Self { path, ..default() } }

	/// Returns a default configuration suitable for watching Cargo projects.
	pub fn default_cargo() -> Self {
		Self {
			filter: GlobFilter::default()
				.with_exclude("*.git*")
				// temp until we get fine grained codegen control
				.with_exclude("*codegen*")
				.with_exclude("*.beet*")
				.with_exclude("*rustc-ice-*")
				.with_exclude("*target*"),
			// avoid short burst refreshing
			debounce: Duration::from_millis(100),
			..default()
		}
	}

	/// Sets the path for the watcher.
	pub fn with_path(mut self, path: AbsPathBuf) -> Self {
		self.path = path;
		self
	}

	/// Sets the filter for the watcher.
	pub fn with_filter(mut self, filter: GlobFilter) -> Self {
		self.filter = filter;
		self
	}

	/// Sets the debounce time for the watcher.
	pub fn with_debounce(mut self, debounce: Duration) -> Self {
		self.debounce = debounce;
		self
	}

	/// Asserts that the watched path exists.
	///
	/// It is not valid to watch a non-existent path; it will never be triggered.
	pub fn assert_path_exists(&self) -> Result {
		if self.path.exists() == false {
			bevybail!(
				"Path does not exist: {}\nOnly existing paths can be watched",
				self.path.display()
			)
		} else {
			Ok(())
		}
	}
}
// TODO kill watcher on remove component
fn start_fs_watcher(mut world: DeferredWorld, cx: HookContext) {
	let entity = cx.entity;
	let watcher = world.entity(entity).get::<FsWatcher>().unwrap().clone();
	world.commands().queue_async(async move |world| {
		watcher.assert_path_exists()?;
		let (tx, rx) = async_channel::unbounded();
		let mut debouncer = new_debouncer(watcher.debounce, None, move |ev| {
			// println!("EV! {:#?}", ev);
			tx.try_send(ev).ok(/* ignore dropped rx, thats allowed */);
		})?;
		debouncer.watch(&watcher.path, RecursiveMode::Recursive)?;

		while let Ok(ev) = rx.recv().await {
			let ev = match ev {
				Ok(ev) => ev,
				Err(errs) => {
					bevybail!("Watch event contains errors: {:?}", errs);
				}
			};
			let Some(ev) = DirEvent::new(ev)?
				.apply_filter(|ev| watcher.filter.passes(&ev.path))
			else {
				// empty after filter
				continue;
			};

			let ev = match (watcher.mutated_only, ev.has_mutate()) {
				(true, false) => {
					// mutated only but contains no mutated
					continue;
				}
				(true, true) => {
					// only send mutated events
					ev.mutated()
				}
				(false, _) => ev,
			};
			world.entity(entity).trigger_target_then(ev).await;
		}
		Ok(())
	})
}


/// An file system event that occurred for a given file or directory.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct PathEvent {
	/// The kind of file system event that occurred.
	pub kind: EventKind,
	/// The path that the event occurred on.
	pub path: AbsPathBuf,
}

impl PathEvent {
	/// Creates a new [`PathEvent`].
	pub fn new(kind: EventKind, path: AbsPathBuf) -> Self {
		Self { kind, path }
	}

	/// Returns `true` if this is a mutation event (create, modify, or remove).
	pub fn mutated(&self) -> bool {
		self.kind.is_create() || self.kind.is_modify() || self.kind.is_remove()
	}

	/// Returns a human-readable display string for this event.
	pub fn display(&self) -> String { format!("{}", self) }
}

impl std::fmt::Display for PathEvent {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}: {}", self.kind, self.path.display())
	}
}

/// Result type for watch events.
pub type WatchEventResult = Result<DirEvent, Vec<notify::Error>>;

/// Collection of [`PathEvent`]s present in a debounced event result.
#[derive(Debug, Default, Deref, EntityTargetEvent)]
pub struct DirEvent {
	events: Vec<PathEvent>,
}

impl std::fmt::Display for DirEvent {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for event in &self.events {
			writeln!(f, "{}", event.display())?;
		}
		Ok(())
	}
}

impl DirEvent {
	/// Creates a new [`DirEvent`] from debounced events.
	pub fn new(events: Vec<DebouncedEvent>) -> Result<Self> {
		Self {
			events: events
				.into_iter()
				.map(|e| {
					let kind = e.kind;
					e.paths
						.iter()
						.map(move |path| {
							let path = AbsPathBuf::new(path)?;
							PathEvent::new(kind.clone(), path).xok()
						})
						.collect::<Vec<_>>()
				})
				.flatten()
				.collect::<Result<Vec<_>>>()?,
		}
		.xok()
	}

	/// Consumes self and returns the inner events.
	pub fn take(self) -> Vec<PathEvent> { self.events }


	/// Returns None if no events match the filter.
	fn apply_filter(
		mut self,
		filter: impl Fn(&PathEvent) -> bool,
	) -> Option<Self> {
		self.events.retain(|e| filter(e));
		if self.events.is_empty() {
			None
		} else {
			Some(self)
		}
	}

	/// Returns `true` if any event matches the predicate.
	pub fn any(&self, func: impl FnMut(&PathEvent) -> bool) -> bool {
		self.events.iter().any(func)
	}

	/// Finds the first event that matches the predicate.
	pub fn find<O>(
		&self,
		func: impl FnMut(&PathEvent) -> Option<O>,
	) -> Option<O> {
		self.events.iter().find_map(func)
	}

	/// Returns `true` if this event contains any mutations.
	///
	/// Equivalent to `is_create() || is_modify() || is_remove()`.
	pub fn has_mutate(&self) -> bool {
		self.has_create() || self.has_modify() || self.has_remove()
	}

	/// Returns a new [`DirEvent`] containing only mutated events.
	pub fn mutated(self) -> Self {
		let events = self
			.events
			.into_iter()
			.filter_map(|e| {
				if e.kind.is_create()
					|| e.kind.is_modify()
					|| e.kind.is_remove()
				{
					Some(e)
				} else {
					None
				}
			})
			.collect();
		Self { events }
	}

	/// Returns a pretty-printed string of mutated events, or `None` if empty.
	pub fn mutated_pretty(self) -> Option<String> {
		let str = self
			.mutated()
			.iter()
			.map(|e| e.display())
			.collect::<Vec<_>>()
			.join("\n");
		if str.is_empty() { None } else { Some(str) }
	}

	/// Returns `true` if any event is an access event.
	pub fn has_access(&self) -> bool {
		self.events.iter().any(|e| e.kind.is_access())
	}

	/// Returns `true` if any event is a create event.
	pub fn has_create(&self) -> bool {
		self.events.iter().any(|e| e.kind.is_create())
	}

	/// Returns `true` if any event is a file creation event.
	pub fn has_create_file(&self) -> bool {
		self.events
			.iter()
			.any(|e| matches!(e.kind, EventKind::Create(CreateKind::File)))
	}

	/// Returns `true` if any event is a directory creation event.
	pub fn has_create_dir(&self) -> bool {
		self.events
			.iter()
			.any(|e| matches!(e.kind, EventKind::Create(CreateKind::Folder)))
	}

	/// Returns `true` if any event is a modify event.
	pub fn has_modify(&self) -> bool {
		self.events.iter().any(|e| e.kind.is_modify())
	}

	/// Returns `true` if any event is a remove event.
	pub fn has_remove(&self) -> bool {
		self.events.iter().any(|e| e.kind.is_remove())
	}

	/// Returns `true` if any event is a file removal event.
	pub fn has_remove_file(&self) -> bool {
		self.events
			.iter()
			.any(|e| matches!(e.kind, EventKind::Remove(RemoveKind::File)))
	}

	/// Returns `true` if any event is a directory removal event.
	pub fn has_remove_dir(&self) -> bool {
		self.events
			.iter()
			.any(|e| matches!(e.kind, EventKind::Remove(RemoveKind::Folder)))
	}

	/// Returns `true` if any event is an "other" event.
	pub fn has_other(&self) -> bool {
		self.events.iter().any(|e| e.kind.is_other())
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// this one is notoriously flaky
	#[crate::test]
	async fn works() {
		let mut app = App::new();
		let tempdir = TempDir::new().unwrap();
		let path = tempdir.path().clone();
		let path2 = path.clone();
		app.add_plugins(AsyncPlugin)
			.spawn(FsWatcher::default().with_path(path.clone()))
			.add_observer(move |ev: On<DirEvent>, mut commands: Commands| {
				for ev in ev.iter() {
					if ev.path.starts_with(&path2) {
						commands.write_message(AppExit::Success);
					}
				}
			});

		// off-thread required for for multi_threaded, not sure why
		std::thread::spawn(move || {
			std::thread::sleep(Duration::from_millis(100));
			fs_ext::write(path.join("foobar.txt"), "foobar").unwrap();
		});
		app.run_async().await.xpect_eq(AppExit::Success);
		// tempdir kept alive until here to prevent cleanup race
		drop(tempdir);
	}
}
