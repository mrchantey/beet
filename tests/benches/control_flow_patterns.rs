//! # Control Flow Patterns Benchmark
//!
//! Benchmarks different ECS control flow patterns for propagating actions through entity hierarchies.
//!
//! ## Patterns Tested
//!
//! 1. **Observer Pattern**: Uses `EntityTargetEvent` with a global observer that manually propagates
//!    events to children. Each entity increments a counter when it receives the event, then triggers
//!    the event on all its children.
//!
//! 2. **Flat System Pattern (Repeated)**: Uses regular systems with `Added<Running>` queries. Systems
//!    remove `Running` from themselves and add it to children. The schedule runs repeatedly until no
//!    more changes occur in a frame, allowing full propagation down the hierarchy.
//!
//! 3. **Flat System Pattern (Single Update)**: Same as above but only runs one schedule update,
//!    showing the cost of a single propagation step for comparison.
//!
//! ## Test Setup
//!
//! - Creates a deep hierarchy of N entities (child chains)
//! - Each entity has a child, forming a chain: root -> child1 -> child2 -> ... -> leafN
//! - Measures time to propagate action from root to all leaves
//!
//! ## Key Findings
//!
//! - **Observer Pattern**: Scales linearly with depth (~0.07µs per entity)
//! - **Flat System (Repeated)**: Much slower, requires multiple schedule runs (50-750x slower)
//! - **Flat System (Single)**: Constant time (~12µs) but only does one step
//!
//! The observer pattern is dramatically more efficient for deep hierarchical propagation since
//! it processes the entire chain in a single schedule run, while the flat system pattern requires
//! depth-many schedule runs to achieve the same result.
//!
//! ## Running
//!
//! ```sh
//! cargo bench --bench control_flow_patterns
//! ```

use beet_core::prelude::*;

const CHAIN_DEPTHS: &[usize] = &[10, 50, 100, 500, 1000];
const ITERATIONS: usize = 100;

// ============================================================================
// Observer Pattern Components
// ============================================================================
//
// Note: The `#[event(propagate = &'static ChildOf)]` attribute enables propagation
// UP the hierarchy (to parents), not down to children. Since we want to cascade
// DOWN the hierarchy, we manually trigger events on children in the observer.

#[derive(Component)]
struct ObserverStepCount(usize);

#[derive(EntityTargetEvent)]
#[event(propagate = &'static ChildOf)]
struct Run;

fn setup_observer_hierarchy(world: &mut World, depth: usize) -> Entity {
	// Add a single global observer for all Run events
	world.add_observer(on_run);

	let root = world.spawn((ObserverStepCount(0), Name::new("Root"))).id();

	let mut current = root;
	for i in 1..depth {
		let child = world
			.spawn((
				ObserverStepCount(0),
				Name::new(format!("Child{}", i)),
				ChildOf(current),
			))
			.id();
		current = child;
	}

	root
}

fn on_run(
	trigger: On<Run>,
	mut query: Query<(&mut ObserverStepCount, Option<&Children>)>,
	mut commands: Commands,
) {
	let entity = trigger.target();
	if let Ok((mut count, children)) = query.get_mut(entity) {
		count.0 += 1;

		// Manually propagate to children
		if let Some(children) = children {
			for child in children.iter() {
				commands.entity(child).trigger_target(Run);
			}
		}
	}
}

// ============================================================================
// Flat System Pattern Components
// ============================================================================

#[derive(Component)]
struct Running;

#[derive(Component)]
struct FlatStepCount(usize);

#[derive(Resource, Default)]
struct StepsThisFrame(usize);

fn setup_flat_hierarchy(world: &mut World, depth: usize) -> Entity {
	let root = world.spawn((FlatStepCount(0), Name::new("Root"))).id();

	let mut current = root;
	for i in 1..depth {
		let child = world
			.spawn((
				FlatStepCount(0),
				Name::new(format!("Child{}", i)),
				ChildOf(current),
			))
			.id();
		current = child;
	}

	root
}

fn propagate_running_system(
	mut commands: Commands,
	mut query: Query<
		(Entity, &mut FlatStepCount, Option<&Children>),
		Added<Running>,
	>,
	mut steps: ResMut<StepsThisFrame>,
) {
	for (entity, mut count, children) in query.iter_mut() {
		count.0 += 1;
		steps.0 += 1;

		// Remove Running from self
		commands.entity(entity).remove::<Running>();

		// Add Running to children
		if let Some(children) = children {
			for child in children.iter() {
				commands.entity(child).insert(Running);
			}
		}
	}
}

fn reset_steps(mut steps: ResMut<StepsThisFrame>) { steps.0 = 0; }

// ============================================================================
// Benchmark Runner
// ============================================================================

fn benchmark_observer_pattern(depth: usize, iterations: usize) -> Duration {
	let mut times = Vec::new();

	for iter in 0..iterations {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);

		let root = setup_observer_hierarchy(app.world_mut(), depth);
		app.update(); // Process commands

		let start = Instant::now();

		// Trigger the event on root
		app.world_mut().entity_mut(root).trigger_target(Run);
		app.update(); // Process the event propagation

		times.push(start.elapsed());

		// Validate on first iteration
		if iter == 0 {
			let total_count: usize = app
				.world_mut()
				.query::<&ObserverStepCount>()
				.iter(app.world())
				.map(|c| c.0)
				.sum();
			if total_count != depth {
				panic!(
					"Observer pattern validation failed: expected {} steps, got {}",
					depth, total_count
				);
			}
		}
	}

	times.iter().sum::<Duration>() / iterations as u32
}

fn benchmark_flat_system(depth: usize, iterations: usize) -> Duration {
	let mut times = Vec::new();

	for iter in 0..iterations {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.insert_resource(StepsThisFrame(0));
		app.add_systems(First, reset_steps);
		app.add_systems(Update, propagate_running_system);

		let root = setup_flat_hierarchy(app.world_mut(), depth);
		app.update(); // Process commands

		let start = Instant::now();

		// Start the propagation
		app.world_mut().entity_mut(root).insert(Running);

		// Run schedule repeatedly until no more steps
		let max_iterations = depth * 2; // Safety limit
		for _ in 0..max_iterations {
			app.update();
			let steps = app.world().resource::<StepsThisFrame>().0;
			if steps == 0 {
				break;
			}
		}

		times.push(start.elapsed());

		// Validate on first iteration
		if iter == 0 {
			let total_count: usize = app
				.world_mut()
				.query::<&FlatStepCount>()
				.iter(app.world())
				.map(|c| c.0)
				.sum();
			if total_count != depth {
				panic!(
					"Flat system validation failed: expected {} steps, got {}",
					depth, total_count
				);
			}
		}
	}

	times.iter().sum::<Duration>() / iterations as u32
}

fn benchmark_flat_system_single_update(
	depth: usize,
	iterations: usize,
) -> Duration {
	let mut times = Vec::new();

	for _ in 0..iterations {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.insert_resource(StepsThisFrame(0));
		app.add_systems(First, reset_steps);
		app.add_systems(Update, propagate_running_system);

		let root = setup_flat_hierarchy(app.world_mut(), depth);
		app.update(); // Process commands

		let start = Instant::now();

		// Start the propagation
		app.world_mut().entity_mut(root).insert(Running);

		// Single update (will only propagate one step)
		app.update();

		times.push(start.elapsed());
	}

	times.iter().sum::<Duration>() / iterations as u32
}

fn main() {
	println!("\n🎯 Control Flow Patterns Benchmark");
	println!("Iterations per depth: {}", ITERATIONS);
	println!("{}", "=".repeat(80));

	for &depth in CHAIN_DEPTHS {
		println!("\n📊 Chain Depth: {}", depth);
		println!("{}", "-".repeat(80));

		// Benchmark observer pattern
		let observer_time = benchmark_observer_pattern(depth, ITERATIONS);
		println!(
			"  ⚡ Observer Pattern:              {:>12.2?} (per run)",
			observer_time
		);

		// Benchmark flat system with repeated schedule
		let flat_repeated_time = benchmark_flat_system(depth, ITERATIONS);
		println!(
			"  🔄 Flat System (repeated):        {:>12.2?} (per run)",
			flat_repeated_time
		);

		// Benchmark flat system single update (for reference)
		let flat_single_time =
			benchmark_flat_system_single_update(depth, ITERATIONS);
		println!(
			"  📦 Flat System (single update):   {:>12.2?} (per run)",
			flat_single_time
		);

		// Calculate ratios
		let repeated_ratio = flat_repeated_time.as_nanos() as f64
			/ observer_time.as_nanos() as f64;
		println!(
			"\n  📈 Flat (repeated) vs Observer: {:.2}x {}",
			repeated_ratio.abs(),
			if repeated_ratio > 1.0 {
				"slower"
			} else {
				"faster"
			}
		);

		let single_ratio = flat_single_time.as_nanos() as f64
			/ observer_time.as_nanos() as f64;
		println!(
			"  📈 Flat (single) vs Observer:   {:.2}x {}",
			single_ratio.abs(),
			if single_ratio > 1.0 {
				"slower"
			} else {
				"faster"
			}
		);
	}

	println!("\n{}", "=".repeat(80));
	println!("✅ Benchmark Complete\n");
}
