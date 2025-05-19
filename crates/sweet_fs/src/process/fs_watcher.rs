use anyhow::Result;
use clap::Parser;
use notify::event::CreateKind;
use notify::event::RemoveKind;
use notify::*;
use notify_debouncer_full::DebounceEventResult;
use notify_debouncer_full::new_debouncer;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::time::Duration;
use sweet_utils::prelude::*;

/// A file watcher with glob patterns. All matches against
/// `include` and `exclude` patterns will be normalized to forward slashes
/// ## Common pitfalls:
/// - If the directory does not exist when the watcher
/// 	starts it will error
/// - If the directory is removed while watching, the
/// 	watcher will silently stop listening
#[derive(Debug, Clone, Parser)]
pub struct FsWatcher {
	/// the path to watch
	#[arg(long, default_value = "./")]
	pub cwd: PathBuf,
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
	/// if true any error will be printed but will not terminate the program
	#[arg(short, long)]
	pub infallible: bool,
}

pub fn parse_duration(s: &str) -> Result<Duration, ParseIntError> {
	s.parse().map(Duration::from_millis)
}

impl Default for FsWatcher {
	fn default() -> Self { Self::parse_from(&[""]) }
}


impl FsWatcher {
	/// It is not valid to watch an empty path, it
	/// will never be triggered!
	pub fn assert_path_exists(&self) -> Result<()> {
		if self.cwd.exists() == false {
			Err(anyhow::anyhow!(
				"Path does not exist: {}\nOnly existing paths can be watched",
				self.cwd.display()
			))
		} else {
			Ok(())
		}
	}

	pub fn watch_blocking(
		&self,
		mut on_change: impl FnMut(WatchEventVec) -> Result<()>,
	) -> Result<()> {
		self.assert_path_exists()?;
		let (tx, rx) = std::sync::mpsc::channel();
		let mut debouncer = new_debouncer(self.debounce, None, move |ev| {
			if let Err(err) = tx.send(ev) {
				eprintln!("{:?}", err);
			}
		})?;
		debouncer.watch(&self.cwd, RecursiveMode::Recursive)?;
		for ev in rx {
			if let Some(ev) = WatchEventVec::new(ev)
				.apply_filter(|ev| self.filter.passes(&ev.path))
			{
				self.handle_on_change_result(on_change(ev))?;
			}
		}
		Ok(())
	}


	/// Watch asynchronously and call [on_change] on each event.
	/// Note that watch events may not actually be fs mutations,
	/// see [WatchEventVec] for more information.
	pub async fn watch_async(
		&self,
		mut on_change: impl FnMut(WatchEventVec) -> Result<()>,
	) -> Result<()> {
		self.assert_path_exists()?;
		let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
		let mut debouncer = new_debouncer(self.debounce, None, move |ev| {
			if let Err(err) = tx.send(ev) {
				eprintln!("{:?}", err);
			}
		})?;
		debouncer.watch(&self.cwd, RecursiveMode::Recursive)?;

		while let Some(ev) = rx.recv().await {
			if let Some(ev) = WatchEventVec::new(ev)
				.apply_filter(|ev| self.filter.passes(&ev.path))
			{
				self.handle_on_change_result(on_change(ev))?;
			}
		}
		Ok(())
	}

	/// Decides how to handle a result from the [on_change] callback
	/// based on the [`infallible`] flag.
	fn handle_on_change_result(&self, result: Result<()>) -> Result<()> {
		match (self.infallible, result) {
			(false, Err(err)) => Err(err),
			(true, Err(err)) => {
				eprintln!("Error: {:?}", err);
				Ok(())
			}
			_ => Ok(()),
		}
	}
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct WatchEvent {
	pub kind: EventKind,
	pub path: PathBuf,
}
impl WatchEvent {
	pub fn new(kind: EventKind, path: impl Into<PathBuf>) -> Self {
		Self {
			kind,
			path: path.into(),
		}
	}
	pub fn display(&self) -> String { format!("{}", self) }
}
impl std::fmt::Display for WatchEvent {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}: {}", self.kind, self.path.display())
	}
}

/// Wrapper for debounced events,
/// queries are match
#[derive(Debug)]
pub struct WatchEventVec {
	pub events: Vec<WatchEvent>,
	pub errors: Vec<Error>,
}
impl WatchEventVec {
	pub fn new(events: DebounceEventResult) -> Self {
		match events {
			Ok(events) => Self {
				events: events
					.into_iter()
					.map(|e| {
						let kind = e.kind;
						e.event
							.paths
							.into_iter()
							.map(move |p| WatchEvent::new(kind.clone(), p))
					})
					.flatten()
					.collect(),
				errors: Vec::new(),
			},
			Err(errors) => Self {
				events: Vec::new(),
				errors,
			},
		}
	}

	/// Returns None if no events match the filter
	pub fn apply_filter(
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
	pub fn mutated(&self) -> Vec<&WatchEvent> {
		self.events
			.iter()
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

	pub fn mutated_pretty(&self) -> Option<String> {
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
