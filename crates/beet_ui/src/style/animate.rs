//! Style transitions: animating resolved style changes over time.
//!
//! The cascade ([`resolve_styles`]) writes *target* values; this module eases
//! the *displayed* value toward them, the CSS `transition` analogue. Elements
//! whose resolved `transition-duration` is nonzero (eg buttons/links via the
//! material `interactive_transition` rule) carry a [`VisualTransition`]: when
//! the cascade retargets their [`VisualStyle`] — a `:hover`/`:focus`/class
//! change, any state at all — the previous displayed style eases into the new
//! one over a bevy [`Timer`], shaped by the resolved [`EaseFunction`].
//!
//! Renderers read [`VisualTransition::current`] when present (see
//! `CharcellNodeData::visual_style`), so the whole pipeline downstream of the
//! cascade is animation-agnostic.
use crate::style::*;
use beet_core::prelude::*;

/// Resolved transition settings for an element, the CSS
/// `transition-duration`/`transition-timing-function` pair.
///
/// Only attached while the duration is nonzero, so the animation system
/// iterates transitioned elements alone.
#[derive(Debug, Clone, PartialEq, Component)]
pub struct TransitionStyle {
	pub duration: Duration,
	pub ease: EaseFunction,
}

impl Default for TransitionStyle {
	fn default() -> Self {
		Self {
			duration: Duration::ZERO,
			// the Material standard easing, mirroring `MotionStandard`
			ease: EaseFunction::CubicInOut,
		}
	}
}

/// The eased, currently-displayed style of a transitioning element.
///
/// Holds the in-flight interpolation state: [`current`](Self::current) is what
/// paint shows, easing from the style at retarget time toward the entity's
/// resolved [`VisualStyle`].
#[derive(Debug, Clone, Component)]
pub struct VisualTransition {
	/// The displayed style this frame.
	pub current: VisualStyle,
	/// The displayed style at the moment of the last retarget.
	from: VisualStyle,
	timer: Timer,
	ease: EaseFunction,
}

impl VisualTransition {
	/// A settled carrier displaying `style`, used at first resolve so an
	/// element's initial appearance never animates in from nothing.
	pub fn snapped(style: VisualStyle) -> Self {
		let mut timer = Timer::new(Duration::ZERO, TimerMode::Once);
		timer.tick(Duration::ZERO);
		Self {
			from: style.clone(),
			current: style,
			timer,
			ease: EaseFunction::Linear,
		}
	}

	/// Whether the transition is still easing toward its target.
	pub fn is_animating(&self) -> bool { !self.timer.is_finished() }

	/// Begin easing from the currently displayed style toward a new target.
	fn retarget(&mut self, transition: &TransitionStyle) {
		self.from = self.current.clone();
		self.timer = Timer::new(transition.duration, TimerMode::Once);
		self.ease = transition.ease;
	}

	/// Advance the eased value toward `target`.
	fn tick(&mut self, delta: Duration, target: &VisualStyle) {
		if self.timer.is_finished() {
			// settled (or zero duration): display the target exactly
			if &self.current != target {
				self.current = target.clone();
			}
			return;
		}
		self.timer.tick(delta);
		let t = EasingCurve::new(0., 1., self.ease)
			.sample_clamped(self.timer.fraction());
		self.current = self.from.mix(target, t);
	}
}

/// The [`PostParseTree`] set easing displayed styles toward the cascade's
/// resolved targets, after [`ResolveStylesSet`] and before any paint set.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct AnimateStylesSet;

/// ECS system: attach, retarget, and tick each transitioned element's
/// [`VisualTransition`].
///
/// A [`Changed<VisualStyle>`] (the cascade writes targets with `set_if_neq`)
/// retargets the ease from the currently displayed style; the bevy [`Timer`]
/// then drives the interpolation each frame.
pub fn animate_visual_transitions(
	time: Res<Time>,
	mut commands: Commands,
	mut query: Query<(
		Entity,
		Ref<VisualStyle>,
		&TransitionStyle,
		Option<&mut VisualTransition>,
	)>,
) {
	for (entity, style, transition, animated) in query.iter_mut() {
		match animated {
			// first resolve: attach settled on the target, no animate-in
			None => {
				commands
					.entity(entity)
					.insert(VisualTransition::snapped(style.as_ref().clone()));
			}
			Some(mut animated) => {
				if style.is_changed() {
					animated.retarget(transition);
				}
				animated.tick(time.delta(), &style);
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::prelude::*;
	use bevy::math::UVec2;

	/// A charcell world with a manually driven [`Time`], a 1s transition on
	/// `.box`, and `.a`/`.b` colour rules. Returns the world and the element.
	fn transition_world() -> (World, Entity) {
		let mut world = CharcellPlugin::world();
		world.init_resource::<Time>();
		world.get_resource_or_init::<RuleSet>().extend_rules(vec![
			Rule::class("box")
				.with_value(common_props::TransitionDurationProp, Duration::from_secs(1))
				.with_value(common_props::TransitionEaseProp, EaseFunction::Linear),
			Rule::class("a")
				.with_value(common_props::ForegroundColor, Color::srgb(0., 0., 0.)),
			Rule::class("b")
				.with_value(common_props::ForegroundColor, Color::srgb(1., 1., 1.)),
		]);
		world.spawn((
			Buffer::new(UVec2::new(8, 2)).into_double_buffer(),
			rsx! { <div class="box a">"x"</div> },
		));
		world.run_schedule(PostParseTree);
		let element = world
			.query_filtered::<Entity, With<Element>>()
			.iter(&world)
			.next()
			.unwrap();
		(world, element)
	}

	fn advance(world: &mut World, delta: Duration) {
		world.resource_mut::<Time>().advance_by(delta);
		world.run_schedule(PostParseTree);
	}

	fn displayed_fg(world: &mut World, entity: Entity) -> Color {
		world
			.get::<VisualTransition>(entity)
			.unwrap()
			.current
			.foreground
			.unwrap()
	}

	/// Retargeting eases the displayed colour over the resolved duration: the
	/// first appearance snaps, a class swap lerps midway at half time and
	/// settles on the target at full time.
	#[beet_core::test]
	fn class_change_eases_foreground() {
		let (mut world, element) = transition_world();
		// first resolve snapped straight to black, no animate-in
		displayed_fg(&mut world, element)
			.xpect_eq(Color::srgb(0., 0., 0.));

		// retarget to white via a class swap
		world.entity_mut(element).insert(Classes::new([
			ClassName::string("box"),
			ClassName::string("b"),
		]));
		advance(&mut world, Duration::ZERO);
		// halfway: the linear ease displays the midpoint grey
		advance(&mut world, Duration::from_millis(500));
		let mid = displayed_fg(&mut world, element).to_srgba();
		mid.red.xpect_close(0.5);
		world
			.get::<VisualTransition>(element)
			.unwrap()
			.is_animating()
			.xpect_true();
		// full time: settled exactly on the target
		advance(&mut world, Duration::from_millis(500));
		displayed_fg(&mut world, element).xpect_eq(Color::srgb(1., 1., 1.));
		world
			.get::<VisualTransition>(element)
			.unwrap()
			.is_animating()
			.xpect_false();
	}

	/// An element with no transition duration never carries a
	/// [`VisualTransition`]; paint reads its resolved style directly.
	#[beet_core::test]
	fn zero_duration_attaches_nothing() {
		let mut world = CharcellPlugin::world();
		world.init_resource::<Time>();
		world.spawn((
			Buffer::new(UVec2::new(8, 2)).into_double_buffer(),
			rsx! { <div>"x"</div> },
		));
		world.run_schedule(PostParseTree);
		world
			.query_filtered::<Entity, With<VisualTransition>>()
			.iter(&world)
			.count()
			.xpect_eq(0);
	}

	/// The painted cell shows the eased colour mid-transition, proving paint
	/// reads the displayed value rather than the resolved target.
	#[beet_core::test]
	fn paint_shows_eased_colour() {
		let (mut world, element) = transition_world();
		world.entity_mut(element).insert(Classes::new([
			ClassName::string("box"),
			ClassName::string("b"),
		]));
		advance(&mut world, Duration::ZERO);
		advance(&mut world, Duration::from_millis(500));
		let buffer = world
			.query::<&DoubleBuffer>()
			.iter(&world)
			.next()
			.unwrap();
		let cell_fg = buffer
			.current_buffer()
			.iter_cells()
			.find(|(_, cell)| cell.symbol_str() == "x")
			.unwrap()
			.1
			.style
			.foreground
			.unwrap()
			.to_srgba();
		cell_fg.red.xpect_close(0.5);
	}
}
