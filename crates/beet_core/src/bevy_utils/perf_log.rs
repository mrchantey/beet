//! Periodic in-process performance reporting, for a long-running beet app.
//!
//! A host's own metrics (ECS/CloudWatch CPU and memory, a `top` reading) say how
//! busy a process is but never why: the schedule loop and the world are opaque
//! from outside. This reports the two numbers that are not, on an interval, into
//! the app's normal log stream: how long a tick takes and how many entities
//! exist. A tick that slows and a count that only climbs are the two shapes a
//! server-side ECS regression takes, and neither is visible to the host until it
//! shows up as spend.

use crate::prelude::*;
use bevy::diagnostic::DiagnosticsStore;
use bevy::diagnostic::EntityCountDiagnosticsPlugin;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;

/// Logs a periodic performance report while it exists, ie
/// `<PerfLog interval="5m"/>` on a deployed site's entry.
///
/// A component rather than a plugin flag so a scene turns reporting on and tunes
/// it, with no rebuild and no env var. Reporting stops when it despawns; with
/// several, the shortest interval wins.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct PerfLog {
	/// How often to report, 5 minutes by default. Each report covers the window
	/// since the previous one.
	pub interval: Duration,
}

impl Default for PerfLog {
	fn default() -> Self {
		Self {
			interval: Duration::from_secs(5 * 60),
		}
	}
}

/// One window's measurements: the schedule's health and the world's size.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct PerfReport {
	/// Mean tick duration over the diagnostic's history, in milliseconds.
	pub mean_tick_ms: f64,
	/// Worst tick in that history, in milliseconds: what a stall looks like when
	/// the mean hides it.
	pub worst_tick_ms: f64,
	/// Live entity count at the end of the window.
	pub entities: f64,
	/// Change in entity count since the previous report. Positive report after
	/// report is the signature of a leak, which a host's memory graph only shows
	/// once it is already expensive.
	pub entity_delta: f64,
}

impl core::fmt::Display for PerfReport {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(
			f,
			"perf: tick {:.2}ms mean, {:.2}ms worst | entities {:.0} ({:+.0})",
			self.mean_tick_ms,
			self.worst_tick_ms,
			self.entities,
			self.entity_delta
		)
	}
}

/// Collects the diagnostics [`PerfLog`] reports, and drives the reporting.
///
/// The measurement is always on (a frame-time and entity-count sample per tick,
/// cheap enough to leave in a release build); the *reporting* waits for a
/// [`PerfLog`], so an app without one logs nothing.
#[derive(Default)]
pub struct PerfLogPlugin;

impl Plugin for PerfLogPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<PerfLog>()
			.init_plugin::<FrameTimeDiagnosticsPlugin>()
			.init_plugin::<EntityCountDiagnosticsPlugin>()
			.add_systems(Update, PerfLog::report);
	}
}

impl PerfLog {
	/// System: emit one report per [`interval`](Self::interval) elapsed.
	fn report(
		time: Res<Time>,
		diagnostics: Res<DiagnosticsStore>,
		logs: Populated<&Self>,
		mut elapsed: Local<Duration>,
		mut previous: Local<Option<f64>>,
	) {
		let Some(interval) = logs.iter().map(|log| log.interval).min() else {
			return;
		};
		*elapsed += time.delta();
		if *elapsed < interval {
			return;
		}
		*elapsed = Duration::ZERO;
		let report = PerfReport::collect(&diagnostics, *previous);
		*previous = Some(report.entities);
		info!("{report}");
	}
}

impl PerfReport {
	/// Read the current window from the diagnostics, taking the entity delta
	/// against `previous` (no delta for the first report).
	fn collect(diagnostics: &DiagnosticsStore, previous: Option<f64>) -> Self {
		let frame_time =
			diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME);
		let entities = diagnostics
			.get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT)
			.and_then(|count| count.value())
			.unwrap_or_default();
		Self {
			mean_tick_ms: frame_time
				.and_then(|frame| frame.average())
				.unwrap_or_default(),
			worst_tick_ms: frame_time
				.map(|frame| frame.values().copied().fold(0., f64::max))
				.unwrap_or_default(),
			entities,
			entity_delta: entities - previous.unwrap_or(entities),
		}
	}
}

#[cfg(test)]
mod test {
	use super::PerfReport;
	use crate::prelude::*;
	use bevy::diagnostic::DiagnosticsStore;
	use bevy::diagnostic::EntityCountDiagnosticsPlugin;

	fn perf_app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, PerfLogPlugin));
		app
	}

	/// The diagnostics a report reads are collected every tick, whether or not
	/// anything is reporting: a `PerfLog` added later still has history.
	#[crate::test]
	fn collects_diagnostics_without_a_perf_log() {
		let mut app = perf_app();
		app.world_mut().spawn_empty();
		app.update();
		app.update();

		app.world()
			.resource::<DiagnosticsStore>()
			.get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT)
			.and_then(|count| count.value())
			.unwrap()
			.xpect_greater_than(0.);
	}

	/// The world's growth between two reports is what the delta carries: the leak
	/// signal, and the one number a host's metrics cannot supply.
	#[crate::test]
	fn reports_entity_growth_between_windows() {
		let mut app = perf_app();
		app.update();
		let collect = |app: &App, previous| {
			PerfReport::collect(
				app.world().resource::<DiagnosticsStore>(),
				previous,
			)
		};
		let first = collect(&app, None);
		// the first report has nothing to compare against
		first.entity_delta.xpect_eq(0.);

		app.world_mut().spawn_batch((0..10).map(|_| ()));
		app.update();

		collect(&app, Some(first.entities))
			.entity_delta
			.xpect_eq(10.);
	}
}
