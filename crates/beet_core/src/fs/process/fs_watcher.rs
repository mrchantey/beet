use crate::prelude::*;
use bevy::prelude::*;
use clap::Parser;
use notify::EventKind;
// #[cfg(not(target_arch = "wasm32"))]
pub use async_channel::Receiver;
pub use async_channel::Sender;
use notify::INotifyWatcher;
use notify::RecursiveMode;
use notify::event::CreateKind;
use notify::event::RemoveKind;
use notify_debouncer_full::DebounceEventResult;
use notify_debouncer_full::Debouncer;
use notify_debouncer_full::NoCache;
use notify_debouncer_full::new_debouncer;
use std::num::ParseIntError;
use std::time::Duration;

/// A file watcher with glob patterns. All matches against
/// `include` and `exclude` patterns will be normalized to forward slashes
/// ## Common pitfalls:
/// - If the directory does not exist when the watcher
/// 	starts it will error
/// - If the directory is removed while watching, the
/// 	watcher will silently stop listening
#[derive(Debug, Clone, Parser, Resource)]
pub struct FsWatcher {
	/// the path to watch
	#[arg(long, default_value = "./")]
	pub cwd: AbsPathBuf,
	#[command(flatten)]
	pub filter: GlobFilter,
	/// debounce time in milliseconds
	#[arg(
		short,
		long="debounce-millis",
		value_parser = parse_duration,
		default_value="50"
	)]
	pub debounce: Duration,
}

pub fn parse_duration(s: &str) -> Result<Duration, ParseIntError> {
	s.parse().map(Duration::from_millis)
}

impl Default for FsWatcher {
	fn default() -> Self { Self::parse_from(&[""]) }
}


impl FsWatcher {
	/// Sets the cwd for the watcher.
	pub fn with_cwd(mut self, cwd: AbsPathBuf) -> Self {
		self.cwd = cwd;
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
		if self.cwd.exists() == false {
			bevybail!(
				"Path does not exist: {}\nOnly existing paths can be watched",
				self.cwd.display()
			)
		} else {
			Ok(())
		}
	}
	/// Return a [`WatchEventReceiver`] that will return
	/// a [`WatchEventVec`] for each event that contains events
	/// matching the [`Self::filter`].
	///
	/// ## Example
	/// ```rust no_run
	/// # use bevy::prelude::*;
	/// # use beet_core::prelude::*;
	/// # async fn foo() -> Result {
	///
	/// let mut rx = FsWatcher::default().watch()?;
	/// while let Some(events) = rx.recv().await? {
	/// 	println!("Received events: {:?}", events);
	/// }
	///
	/// # Ok(()) }
	/// ```
	pub fn watch(&self) -> Result<WatchEventReceiver> {
		#[cfg(target_arch = "wasm32")]
		panic!("File watching is not supported on wasm32");
		self.assert_path_exists()?;
		#[cfg(not(target_arch = "wasm32"))]
		let (tx, rx) = async_channel::unbounded();
		let mut debouncer = new_debouncer(self.debounce, None, move |ev| {
			if let Err(err) = tx.try_send(ev) {
				eprintln!("{:?}", err);
			}
		})?;
		debouncer.watch(&self.cwd, RecursiveMode::Recursive)?;

		#[cfg(not(target_arch = "wasm32"))]
		return Ok(WatchEventReceiver {
			rx,
			_tx: debouncer,
			filter: self.filter.clone(),
		});

		#[cfg(target_arch = "wasm32")]
		unreachable!();
	}
}
// TODO async iterator when stablizes
// https://doc.rust-lang.org/std/async_iter/trait.AsyncIterator.html
#[cfg(not(target_arch = "wasm32"))]
pub struct WatchEventReceiver {
	rx: Receiver<DebounceEventResult>,
	filter: GlobFilter,
	// keep reference to debouncer so it does not get dropped
	_tx: Debouncer<INotifyWatcher, NoCache>,
}

#[cfg(not(target_arch = "wasm32"))]
impl WatchEventReceiver {
	pub async fn recv(&mut self) -> Result<Option<WatchEventVec>> {
		while let Ok(ev) = self.rx.recv().await {
			match WatchEventVec::new(ev)?
				.apply_filter(|ev| self.filter.passes(&ev.path))
			{
				Some(ev_vec) => {
					// receieved events that matches filter
					return Ok(Some(ev_vec));
				}
				// event received but did not match filter so keep waiting
				None => continue,
			}
		}
		// done receiving events
		Ok(None)
	}
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "bevy", derive(bevy::prelude::Event))]
pub struct WatchEvent {
	pub kind: EventKind,
	pub path: AbsPathBuf,
}
impl WatchEvent {
	pub fn new(kind: EventKind, path: AbsPathBuf) -> Self {
		Self { kind, path }
	}
	pub fn mutated(&self) -> bool {
		self.kind.is_create() || self.kind.is_modify() || self.kind.is_remove()
	}
	pub fn display(&self) -> String { format!("{}", self) }
}
impl std::fmt::Display for WatchEvent {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}: {}", self.kind, self.path.display())
	}
}

pub type WatchEventResult = Result<WatchEventVec, Vec<notify::Error>>;

/// Wrapper for debounced events,
/// queries are match
#[derive(Debug, Default)]
pub struct WatchEventVec {
	events: Vec<WatchEvent>,
}
impl std::ops::Deref for WatchEventVec {
	type Target = Vec<WatchEvent>;
	fn deref(&self) -> &Self::Target { &self.events }
}

impl WatchEventVec {
	pub fn new(events: DebounceEventResult) -> Result<Self> {
		let events = match events {
			Ok(events) => events,
			Err(errors) => {
				bevybail!("Watch event contains errors: {:?}", errors)
			}
		};

		Self {
			events: events
				.into_iter()
				.map(|e| {
					let kind = e.kind;
					e.paths
						.iter()
						.map(move |path| {
							let path = AbsPathBuf::new(path)?;
							WatchEvent::new(kind.clone(), path).xok()
						})
						.collect::<Vec<_>>()
				})
				.flatten()
				.collect::<Result<Vec<_>>>()?,
		}
		.xok()
	}
	pub fn take(self) -> Vec<WatchEvent> { self.events }


	/// Returns None if no events match the filter
	fn apply_filter(
		mut self,
		filter: impl Fn(&WatchEvent) -> bool,
	) -> Option<Self> {
		self.events.retain(|e| filter(e));
		if self.events.is_empty() {
			None
		} else {
			Some(self)
		}
	}

	pub fn any(&self, func: impl FnMut(&WatchEvent) -> bool) -> bool {
		self.events.iter().any(func)
	}
	pub fn find<O>(
		&self,
		func: impl FnMut(&WatchEvent) -> Option<O>,
	) -> Option<O> {
		self.events.iter().find_map(func)
	}
	/// equivilent to `is_create() || is_modify() || is_remove()`
	pub fn has_mutate(&self) -> bool {
		self.has_create() || self.has_modify() || self.has_remove()
	}
	pub fn mutated(self) -> Vec<WatchEvent> {
		self.events
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
			.collect()
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
	use bevy::prelude::*;
	use notify::EventKind;
	use notify::event::CreateKind;
	use sweet::prelude::*;
	use tempfile::tempdir;

	#[sweet::test]
	async fn works() -> Result {
		let tmp_dir = tempdir()?;
		let mut rx = FsWatcher {
			cwd: AbsPathBuf::new(tmp_dir.path().to_path_buf()).unwrap(),
			..Default::default()
		}
		.watch()?;

		let file_path = tmp_dir.path().join("foo.txt");
		fs_ext::write(&file_path, "hello")?;

		// does not hang
		let ev = rx.recv().await?.unwrap();

		ev[0].kind.xpect_eq(EventKind::Create(CreateKind::File));
		ev[0].path.as_ref().xpect_eq(file_path);

		Ok(())
	}
}
