//! `<RequireCfg>`: the cfg condition asserted rather than applied.

use crate::prelude::*;

/// Declares a cfg condition the running binary must satisfy for this document
/// to work at all, failing the load when it does not.
///
/// ```html
/// <RequireCfg cfg="feature:infra && feature:extra"/>
/// ```
///
/// The condition is a plain string attribute, matching `bx:cfg`, so the entry
/// pre-scan can read it from raw markup before any registry exists. That
/// matters: a requirement has to be able to fire for an entry whose tree cannot
/// build at all, which is exactly the case it is there to report.
///
/// The third consumer of the one condition grammar, and the only one that
/// refuses rather than adapts. The other two are `bx:cfg`, which excludes a
/// branch whose condition is false, and [`CfgExcluded`], the tombstone it
/// leaves behind. Use this where there is no sensible reduced document: an
/// entry whose whole purpose is the thing that is missing.
///
/// Prefer `bx:cfg` wherever the document CAN still do something useful without
/// the branch. A refusal is right only when it cannot.
///
/// Reporting goes through [`BuildCondition::explain`], which walks a false
/// condition without short-circuiting, so a rebuild fixes everything the error
/// named rather than revealing the next missing item one run at a time.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RequireCfg {
	/// The condition source, ie `feature:infra && feature:extra`.
	pub cfg: SmolStr,
}

impl RequireCfg {
	/// Require `condition`, the code counterpart of the markup tag.
	pub fn new(condition: impl Into<SmolStr>) -> Self {
		Self { cfg: condition.into() }
	}

	/// Observer: validate an inserted [`RequireCfg`] against this build,
	/// logging every unmet atom and exiting non-zero.
	///
	/// Failures across several declarations inserted in the same frame all
	/// report before the exit processes in `Last`, so one run names everything.
	pub fn check_on_insert(
		ev: On<Insert, RequireCfg>,
		mut commands: Commands,
	) -> Result {
		let entity = ev.entity;
		commands.queue(move |world: &mut World| -> Result {
			let Some(require) = world.entity(entity).get::<RequireCfg>().cloned()
			else {
				return Ok(());
			};
			let condition = BuildCondition::parse(&require.cfg)?;
			let conditions =
				world.get_resource_or_init::<BsxConditions>().clone_seam();
			let registrations = world
				.with_state::<Query<&CrateRegistration>, _>(|query| {
					query.iter().cloned().collect::<Vec<_>>()
				});
			let cx = ConditionCx {
				world,
				registrations: &registrations,
			};
			let failures = condition.explain(&conditions, &cx)?;
			if failures.is_empty() {
				return Ok(());
			}
			error!(
				"this binary does not satisfy `<RequireCfg cfg=\"{}\"/>`, unmet:\n  {}\n\
				rebuild with what it names, ie `cargo install --path \
				crates/beet-cli --all-features`",
				require.cfg,
				failures.join("\n  ")
			);
			world.write_message(AppExit::error());
			Ok(())
		});
		Ok(())
	}
}

/// Registers the cfg-requirement components and the insert-time assertion, so
/// an entry's `<RequireCfg/>` verifies this build satisfies it, and a
/// `bx:cfg` tombstone is spawnable. Always on via `BeetPlugins`.
#[derive(Default)]
pub struct CrateCheckPlugin;

impl Plugin for CrateCheckPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<CrateRegistration>()
			.register_type::<RequireCfg>()
			.register_type::<CfgExcluded>()
			.add_observer(RequireCfg::check_on_insert);
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// A world whose primary registration compiles `sockets` at `0.0.9`, plus a
	/// named crate, the shape a requirement is checked against.
	fn world() -> World {
		let mut world = World::new();
		world.init_resource::<BsxConditions>();
		// the assertion itself, plus the channel it exits through
		world.init_resource::<Messages<AppExit>>();
		world.add_observer(RequireCfg::check_on_insert);
		world.spawn(
			CrateRegistration::new("beet-cli", "0.0.9")
				.with_feature("sockets")
				.with_skip_prefix(),
		);
		world.spawn(
			CrateRegistration::new("beet_esp", "0.5.3").with_feature("alvik"),
		);
		world
	}

	/// Every requirement the retired `<CrateCheck features versions>` could
	/// express, now as one condition: unprefixed and `crate/feature` features,
	/// unprefixed and `crate@version` versions.
	#[crate::test]
	fn covers_the_feature_and_version_vocabulary() {
		let world = world();
		let registrations = [
			CrateRegistration::new("beet-cli", "0.0.9")
				.with_feature("sockets")
				.with_skip_prefix(),
			CrateRegistration::new("beet_esp", "0.5.3").with_feature("alvik"),
		];
		let conditions = world.resource::<BsxConditions>();
		let cx = ConditionCx {
			world: &world,
			registrations: &registrations,
		};
		let holds = |source: &str| {
			BuildCondition::parse(source)
				.unwrap()
				.evaluate(conditions, &cx)
				.unwrap()
		};
		holds("feature:sockets").xpect_true();
		holds("feature:beet_esp/alvik").xpect_true();
		holds("version:0.0.9").xpect_true();
		holds("version:beet_esp@0.5.3").xpect_true();
		// a newer compiled version satisfies a minimum
		holds("version:0.0.1").xpect_true();
		// ..and the misses
		holds("feature:winit").xpect_false();
		holds("feature:beet_esp/other").xpect_false();
		holds("feature:nosuchcrate/alvik").xpect_false();
		holds("version:9.0.0").xpect_false();
		holds("version:beet_esp@9.0.0").xpect_false();
	}

	/// A satisfied requirement is silent and does not exit.
	#[crate::test]
	fn passes_when_satisfied() {
		let mut world = world();
		world.spawn(RequireCfg::new("feature:sockets && version:0.0.9"));
		world.flush();
		world
			.resource::<Messages<AppExit>>()
			.iter_current_update_messages()
			.count()
			.xpect_eq(0);
	}

	/// An unsatisfied one writes an error exit, and `explain` means the log
	/// named both misses rather than only the first.
	#[crate::test]
	fn exits_when_unsatisfied() {
		let mut world = world();
		world.spawn(RequireCfg::new(
			"feature:sockets && feature:winit && version:9.0.0",
		));
		world.flush();
		world
			.resource::<Messages<AppExit>>()
			.iter_current_update_messages()
			.any(|exit| exit.is_error())
			.xpect_true();
	}
}
