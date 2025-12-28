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

/// A file watcher with glob patterns. All matches against
/// `include` and `exclude` patterns will be normalized to forward slashes
/// ## Common pitfalls:
/// - If the directory does not exist when the watcher
/// 	starts it will error
/// - If the [`Self::path`] is removed while watching, the
/// 	watcher will silently stop listening
#[derive(Debug, Clone, Component)]
#[component(on_add=start_fs_watcher)]
pub struct FsWatcher {
	/// the path to watch
	pub path: AbsPathBuf,
	/// glob filter for paths to include/exclude
	pub filter: GlobFilter,
	/// debounce time in milliseconds
	pub debounce: Duration,
	/// only send events that mutated paths
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


pub fn parse_duration(s: &str) -> Result<Duration, ParseIntError> {
	s.parse().map(Duration::from_millis)
}

impl FsWatcher {
	pub fn new(path: AbsPathBuf) -> Self { Self { path, ..default() } }

	pub fn default_cargo() -> Self {
		Self {
			filter: GlobFilter::default()
				.with_exclude("*.git*")
				// temp until we get fine grained codegen control
				.with_exclude("*codegen*")
				.with_exclude("*target*"),
			// avoid short burst refreshing
			debounce: Duration::from_millis(100),
			..default()
		}
	}

	/// Sets the cwd for the watcher.
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

	/// It is not valid to watch an empty path, it
	/// will never be triggered!
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
			world.entity(entity).trigger_target(ev).await;
		}
		Ok(())
	})
}


/// An fs event that occured for a given file or directory.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct PathEvent {
	/// The kind of fs event that occurred
	pub kind: EventKind,
	/// The path that the event occurred on
	pub path: AbsPathBuf,
}
impl PathEvent {
	pub fn new(kind: EventKind, path: AbsPathBuf) -> Self {
		Self { kind, path }
	}
	pub fn mutated(&self) -> bool {
		self.kind.is_create() || self.kind.is_modify() || self.kind.is_remove()
	}
	pub fn display(&self) -> String { format!("{}", self) }
}
impl std::fmt::Display for PathEvent {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}: {}", self.kind, self.path.display())
	}
}

pub type WatchEventResult = Result<DirEvent, Vec<notify::Error>>;

/// Collection of each [`PathEvent`] present in a given [`DebounceEventResult`]
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
	pub fn take(self) -> Vec<PathEvent> { self.events }


	/// Returns None if no events match the filter
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

	pub fn any(&self, func: impl FnMut(&PathEvent) -> bool) -> bool {
		self.events.iter().any(func)
	}
	pub fn find<O>(
		&self,
		func: impl FnMut(&PathEvent) -> Option<O>,
	) -> Option<O> {
		self.events.iter().find_map(func)
	}
	/// equivilent to `is_create() || is_modify() || is_remove()`
	pub fn has_mutate(&self) -> bool {
		self.has_create() || self.has_modify() || self.has_remove()
	}
	/// Returns a new DirEvent containing only mutated events
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

	pub fn mutated_pretty(self) -> Option<String> {
		let str = self
			.mutated()
			.iter()
			.map(|e| e.display())
			.collect::<Vec<_>>()
			.join("\n");
		if str.is_empty() { None } else { Some(str) }
	}

	pub fn has_access(&self) -> bool {
		self.events.iter().any(|e| e.kind.is_access())
	}
	pub fn has_create(&self) -> bool {
		self.events.iter().any(|e| e.kind.is_create())
	}
	pub fn has_create_file(&self) -> bool {
		self.events
			.iter()
			.any(|e| matches!(e.kind, EventKind::Create(CreateKind::File)))
	}
	pub fn has_create_dir(&self) -> bool {
		self.events
			.iter()
			.any(|e| matches!(e.kind, EventKind::Create(CreateKind::Folder)))
	}
	pub fn has_modify(&self) -> bool {
		self.events.iter().any(|e| e.kind.is_modify())
	}
	pub fn has_remove(&self) -> bool {
		self.events.iter().any(|e| e.kind.is_remove())
	}
	pub fn has_remove_file(&self) -> bool {
		self.events
			.iter()
			.any(|e| matches!(e.kind, EventKind::Remove(RemoveKind::File)))
	}
	pub fn has_remove_dir(&self) -> bool {
		self.events
			.iter()
			.any(|e| matches!(e.kind, EventKind::Remove(RemoveKind::Folder)))
	}
	pub fn has_other(&self) -> bool {
		self.events.iter().any(|e| e.kind.is_other())
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
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
			std::thread::sleep(Duration::from_millis(1));
			fs_ext::write(path.join("foobar.txt"), "foobar").unwrap();
		});
		app.run_async().await.xpect_eq(AppExit::Success);
		// tempdir kept alive until here to prevent cleanup race
		drop(tempdir);
	}
}
