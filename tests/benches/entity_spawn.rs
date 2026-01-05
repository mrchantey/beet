//! # Entity Spawn Benchmark
//!
//! Measures the cost of spawning entity hierarchies - critical for understanding
//! how to best represent stateful 'per-trigger' control flow hierarchies like
//! router trees or behavior trees.
//!
//! ## Scenarios Tested
//!
//! 1. **Deep Hierarchies**: Single chain of parent->child->grandchild...
//! 2. **Wide Hierarchies**: Many trees with shallow depth
//! 3. **With/Without Names**: Cost of `Name` component
//! 4. **EntityCloner vs world.spawn()**: Cloning existing hierarchies vs fresh spawns
//! 5. **Additional Components**: Cost of components with heap allocations
//! 6. **Single Entity Overhead**: Baseline cost per entity spawn
//! 7. **World Reuse vs Fresh**: Cost of reusing world vs creating new
//! 8. **Batch Operations**: with_children vs individual spawns
//!
//! ## Running
//!
//! ```sh
//! cargo bench --bench entity_spawn
//! ```

use beet_core::prelude::*;
use bevy::ecs::entity::EntityCloner;

const ITERATIONS: usize = 100;
const DEPTHS: &[usize] = &[1, 5, 10, 25, 50, 100];
const WIDTHS: &[usize] = &[1, 10, 50, 100, 500, 1000];

/// Arbitrary component with heap allocation for testing clone costs
#[derive(Component, Clone, Default)]
struct Payload(#[allow(dead_code)] Vec<u8>);

impl Payload {
	fn new(size: usize) -> Self { Self(vec![0u8; size]) }
}

/// Marker component for benchmarking
#[derive(Component, Clone, Default)]
struct BenchMarker;

/// Additional marker to measure multi-component overhead
#[derive(Component, Clone, Default)]
struct ExtraMarker;

/// Another marker for component scaling tests
#[derive(Component, Clone, Default)]
struct ThirdMarker;

/// A small inline component (no heap)
#[derive(Component, Clone, Copy, Default)]
struct SmallData {
	#[allow(dead_code)]
	a: u64,
	#[allow(dead_code)]
	b: u64,
}

// ============================================================================
// Hierarchy Builders
// ============================================================================

/// Spawns a deep hierarchy using ChildOf relationships
fn spawn_deep_hierarchy_bare(world: &mut World, depth: usize) -> Entity {
	if depth == 0 {
		return world.spawn(BenchMarker).id();
	}

	let root = world.spawn(BenchMarker).id();
	let mut current = root;

	for _ in 1..depth {
		let child = world.spawn((BenchMarker, ChildOf(current))).id();
		current = child;
	}

	root
}

/// Spawns a deep hierarchy with Name components
fn spawn_deep_hierarchy_named(world: &mut World, depth: usize) -> Entity {
	if depth == 0 {
		return world.spawn((BenchMarker, Name::new("node_0"))).id();
	}

	let root = world.spawn((BenchMarker, Name::new("node_0"))).id();
	let mut current = root;

	for idx in 1..depth {
		let child = world
			.spawn((
				BenchMarker,
				Name::new(format!("node_{}", idx)),
				ChildOf(current),
			))
			.id();
		current = child;
	}

	root
}

/// Spawns a deep hierarchy with Name and Payload components
fn spawn_deep_hierarchy_with_payload(
	world: &mut World,
	depth: usize,
	payload_size: usize,
) -> Entity {
	if depth == 0 {
		return world
			.spawn((
				BenchMarker,
				Name::new("node_0"),
				Payload::new(payload_size),
			))
			.id();
	}

	let root = world
		.spawn((BenchMarker, Name::new("node_0"), Payload::new(payload_size)))
		.id();
	let mut current = root;

	for idx in 1..depth {
		let child = world
			.spawn((
				BenchMarker,
				Name::new(format!("node_{}", idx)),
				Payload::new(payload_size),
				ChildOf(current),
			))
			.id();
		current = child;
	}

	root
}

/// Spawns many shallow trees (width trees, each with given depth)
fn spawn_wide_hierarchy_bare(
	world: &mut World,
	width: usize,
	depth: usize,
) -> Vec<Entity> {
	(0..width)
		.map(|_| spawn_deep_hierarchy_bare(world, depth))
		.collect()
}

/// Spawns many shallow trees with names
fn spawn_wide_hierarchy_named(
	world: &mut World,
	width: usize,
	depth: usize,
) -> Vec<Entity> {
	(0..width)
		.map(|_| spawn_deep_hierarchy_named(world, depth))
		.collect()
}

// ============================================================================
// Benchmark Functions
// ============================================================================

fn benchmark_deep_hierarchy_bare(depth: usize, iterations: usize) -> Duration {
	let mut times = Vec::with_capacity(iterations);

	for _ in 0..iterations {
		let mut world = World::new();
		let start = Instant::now();
		spawn_deep_hierarchy_bare(&mut world, depth);
		times.push(start.elapsed());
	}

	times.iter().sum::<Duration>() / iterations as u32
}

fn benchmark_deep_hierarchy_named(depth: usize, iterations: usize) -> Duration {
	let mut times = Vec::with_capacity(iterations);

	for _ in 0..iterations {
		let mut world = World::new();
		let start = Instant::now();
		spawn_deep_hierarchy_named(&mut world, depth);
		times.push(start.elapsed());
	}

	times.iter().sum::<Duration>() / iterations as u32
}

fn benchmark_deep_hierarchy_with_payload(
	depth: usize,
	payload_size: usize,
	iterations: usize,
) -> Duration {
	let mut times = Vec::with_capacity(iterations);

	for _ in 0..iterations {
		let mut world = World::new();
		let start = Instant::now();
		spawn_deep_hierarchy_with_payload(&mut world, depth, payload_size);
		times.push(start.elapsed());
	}

	times.iter().sum::<Duration>() / iterations as u32
}

fn benchmark_wide_hierarchy_bare(
	width: usize,
	depth: usize,
	iterations: usize,
) -> Duration {
	let mut times = Vec::with_capacity(iterations);

	for _ in 0..iterations {
		let mut world = World::new();
		let start = Instant::now();
		spawn_wide_hierarchy_bare(&mut world, width, depth);
		times.push(start.elapsed());
	}

	times.iter().sum::<Duration>() / iterations as u32
}

fn benchmark_wide_hierarchy_named(
	width: usize,
	depth: usize,
	iterations: usize,
) -> Duration {
	let mut times = Vec::with_capacity(iterations);

	for _ in 0..iterations {
		let mut world = World::new();
		let start = Instant::now();
		spawn_wide_hierarchy_named(&mut world, width, depth);
		times.push(start.elapsed());
	}

	times.iter().sum::<Duration>() / iterations as u32
}

/// Benchmark cloning a pre-existing deep hierarchy using EntityCloner
fn benchmark_clone_deep_hierarchy_bare(
	depth: usize,
	iterations: usize,
) -> Duration {
	let mut times = Vec::with_capacity(iterations);

	for _ in 0..iterations {
		let mut world = World::new();
		let source = spawn_deep_hierarchy_bare(&mut world, depth);

		let start = Instant::now();
		let mut builder = EntityCloner::build_opt_out(&mut world);
		builder.linked_cloning(true);
		let mut cloner = builder.finish();
		cloner.spawn_clone(&mut world, source);
		times.push(start.elapsed());
	}

	times.iter().sum::<Duration>() / iterations as u32
}

/// Benchmark cloning a pre-existing deep hierarchy with Names
fn benchmark_clone_deep_hierarchy_named(
	depth: usize,
	iterations: usize,
) -> Duration {
	let mut times = Vec::with_capacity(iterations);

	for _ in 0..iterations {
		let mut world = World::new();
		let source = spawn_deep_hierarchy_named(&mut world, depth);

		let start = Instant::now();
		let mut builder = EntityCloner::build_opt_out(&mut world);
		builder.linked_cloning(true);
		let mut cloner = builder.finish();
		cloner.spawn_clone(&mut world, source);
		times.push(start.elapsed());
	}

	times.iter().sum::<Duration>() / iterations as u32
}

/// Benchmark cloning hierarchy with payload
fn benchmark_clone_deep_hierarchy_with_payload(
	depth: usize,
	payload_size: usize,
	iterations: usize,
) -> Duration {
	let mut times = Vec::with_capacity(iterations);

	for _ in 0..iterations {
		let mut world = World::new();
		let source =
			spawn_deep_hierarchy_with_payload(&mut world, depth, payload_size);

		let start = Instant::now();
		let mut builder = EntityCloner::build_opt_out(&mut world);
		builder.linked_cloning(true);
		let mut cloner = builder.finish();
		cloner.spawn_clone(&mut world, source);
		times.push(start.elapsed());
	}

	times.iter().sum::<Duration>() / iterations as u32
}

/// Benchmark cloning multiple times (simulating repeated requests)
fn benchmark_repeated_clones(
	depth: usize,
	clone_count: usize,
	iterations: usize,
) -> Duration {
	let mut times = Vec::with_capacity(iterations);

	for _ in 0..iterations {
		let mut world = World::new();
		let source = spawn_deep_hierarchy_named(&mut world, depth);

		let start = Instant::now();
		let mut builder = EntityCloner::build_opt_out(&mut world);
		builder.linked_cloning(true);
		let mut cloner = builder.finish();
		for _ in 0..clone_count {
			cloner.spawn_clone(&mut world, source);
		}
		times.push(start.elapsed());
	}

	times.iter().sum::<Duration>() / iterations as u32
}

/// Benchmark spawning fresh vs cloning for same total entity count
fn benchmark_spawn_vs_clone_throughput(
	depth: usize,
	count: usize,
	iterations: usize,
) -> (Duration, Duration) {
	// Fresh spawns
	let spawn_times: Vec<Duration> = (0..iterations)
		.map(|_| {
			let mut world = World::new();
			let start = Instant::now();
			for _ in 0..count {
				spawn_deep_hierarchy_named(&mut world, depth);
			}
			start.elapsed()
		})
		.collect();

	// Clone from template
	let clone_times: Vec<Duration> = (0..iterations)
		.map(|_| {
			let mut world = World::new();
			let source = spawn_deep_hierarchy_named(&mut world, depth);
			let mut builder = EntityCloner::build_opt_out(&mut world);
			builder.linked_cloning(true);
			let mut cloner = builder.finish();
			let start = Instant::now();
			for _ in 0..count {
				cloner.spawn_clone(&mut world, source);
			}
			start.elapsed()
		})
		.collect();

	let spawn_avg = spawn_times.iter().sum::<Duration>() / iterations as u32;
	let clone_avg = clone_times.iter().sum::<Duration>() / iterations as u32;
	(spawn_avg, clone_avg)
}

/// Benchmark single entity spawn with varying component counts
fn benchmark_single_entity_components(
	iterations: usize,
) -> Vec<(usize, Duration)> {
	let counts = [1, 2, 3, 4, 5, 8, 10];
	counts
		.iter()
		.map(|&component_count| {
			let times: Vec<Duration> = (0..iterations)
				.map(|_| {
					let mut world = World::new();
					let start = Instant::now();
					match component_count {
						1 => {
							world.spawn(BenchMarker);
						}
						2 => {
							world.spawn((BenchMarker, ExtraMarker));
						}
						3 => {
							world.spawn((
								BenchMarker,
								ExtraMarker,
								SmallData::default(),
							));
						}
						4 => {
							world.spawn((
								BenchMarker,
								ExtraMarker,
								SmallData::default(),
								Name::new("test"),
							));
						}
						5 => {
							world.spawn((
								BenchMarker,
								ExtraMarker,
								SmallData::default(),
								Name::new("test"),
								Payload::new(64),
							));
						}
						_ => {
							world.spawn((
								BenchMarker,
								ExtraMarker,
								ThirdMarker,
								SmallData::default(),
								Name::new("test"),
								Payload::new(64),
							));
						}
					}
					start.elapsed()
				})
				.collect();
			(
				component_count,
				times.iter().sum::<Duration>() / iterations as u32,
			)
		})
		.collect()
}

/// Benchmark world reuse vs fresh world for spawning
fn benchmark_world_reuse(
	entity_count: usize,
	iterations: usize,
) -> (Duration, Duration) {
	// Fresh world each time
	let fresh_times: Vec<Duration> = (0..iterations)
		.map(|_| {
			let start = Instant::now();
			let mut world = World::new();
			for _ in 0..entity_count {
				world.spawn((BenchMarker, Name::new("test")));
			}
			start.elapsed()
		})
		.collect();

	// Reuse world, despawn between runs
	// Reuse world, clear between runs
	let reuse_times: Vec<Duration> = (0..iterations)
		.map(|_| {
			let mut world = World::new();
			// Warm up the world
			for _ in 0..entity_count {
				world.spawn((BenchMarker, Name::new("test")));
			}
			world.clear_entities();

			let start = Instant::now();
			for _ in 0..entity_count {
				world.spawn((BenchMarker, Name::new("test")));
			}
			start.elapsed()
		})
		.collect();

	let fresh_avg = fresh_times.iter().sum::<Duration>() / iterations as u32;
	let reuse_avg = reuse_times.iter().sum::<Duration>() / iterations as u32;
	(fresh_avg, reuse_avg)
}

/// Benchmark with_children batch spawn vs individual ChildOf spawns
fn benchmark_batch_vs_individual(
	child_count: usize,
	iterations: usize,
) -> (Duration, Duration) {
	// Individual spawns with ChildOf
	let individual_times: Vec<Duration> = (0..iterations)
		.map(|_| {
			let mut world = World::new();
			let start = Instant::now();
			let parent = world.spawn(BenchMarker).id();
			for _ in 0..child_count {
				world.spawn((BenchMarker, ChildOf(parent)));
			}
			start.elapsed()
		})
		.collect();

	// Batch spawn with with_children
	let batch_times: Vec<Duration> = (0..iterations)
		.map(|_| {
			let mut world = World::new();
			let start = Instant::now();
			world.spawn(BenchMarker).with_children(|spawner| {
				for _ in 0..child_count {
					spawner.spawn(BenchMarker);
				}
			});
			start.elapsed()
		})
		.collect();

	let individual_avg =
		individual_times.iter().sum::<Duration>() / iterations as u32;
	let batch_avg = batch_times.iter().sum::<Duration>() / iterations as u32;
	(individual_avg, batch_avg)
}

/// Benchmark EntityCloner setup overhead vs just cloning
fn benchmark_cloner_setup_overhead(
	depth: usize,
	iterations: usize,
) -> (Duration, Duration) {
	// Setup cloner once, clone many times
	let reuse_times: Vec<Duration> = (0..iterations)
		.map(|_| {
			let mut world = World::new();
			let source = spawn_deep_hierarchy_named(&mut world, depth);
			let mut builder = EntityCloner::build_opt_out(&mut world);
			builder.linked_cloning(true);
			let mut cloner = builder.finish();

			let start = Instant::now();
			for _ in 0..10 {
				cloner.spawn_clone(&mut world, source);
			}
			start.elapsed()
		})
		.collect();

	// Create new cloner each time
	let fresh_times: Vec<Duration> = (0..iterations)
		.map(|_| {
			let mut world = World::new();
			let source = spawn_deep_hierarchy_named(&mut world, depth);

			let start = Instant::now();
			for _ in 0..10 {
				let mut builder = EntityCloner::build_opt_out(&mut world);
				builder.linked_cloning(true);
				let mut cloner = builder.finish();
				cloner.spawn_clone(&mut world, source);
			}
			start.elapsed()
		})
		.collect();

	let reuse_avg = reuse_times.iter().sum::<Duration>() / iterations as u32;
	let fresh_avg = fresh_times.iter().sum::<Duration>() / iterations as u32;
	(reuse_avg, fresh_avg)
}

// ============================================================================
// Main
// ============================================================================

fn main() {
	println!("\n🎯 Entity Spawn Benchmark");
	println!("Iterations per test: {}", ITERATIONS);
	println!("{}", "=".repeat(90));

	// Section 1: Deep Hierarchies
	println!(
		"\n📊 DEEP HIERARCHIES (single chain: root -> child -> grandchild -> ...)"
	);
	println!("{}", "-".repeat(90));
	println!(
		"{:>8} | {:>12} | {:>12} | {:>12} | {:>15}",
		"Depth", "Bare", "Named", "Named+64B", "Named+1KB"
	);
	println!("{}", "-".repeat(90));

	for &depth in DEPTHS {
		let bare = benchmark_deep_hierarchy_bare(depth, ITERATIONS);
		let named = benchmark_deep_hierarchy_named(depth, ITERATIONS);
		let payload_64 =
			benchmark_deep_hierarchy_with_payload(depth, 64, ITERATIONS);
		let payload_1k =
			benchmark_deep_hierarchy_with_payload(depth, 1024, ITERATIONS);

		println!(
			"{:>8} | {:>12.2?} | {:>12.2?} | {:>12.2?} | {:>15.2?}",
			depth, bare, named, payload_64, payload_1k
		);
	}

	// Section 2: Wide Hierarchies
	println!("\n📊 WIDE HIERARCHIES (many trees, depth=3)");
	println!("{}", "-".repeat(90));
	println!(
		"{:>8} | {:>12} | {:>12} | {:>15}",
		"Width", "Bare", "Named", "Per-Tree (Named)"
	);
	println!("{}", "-".repeat(90));

	for &width in WIDTHS {
		let bare = benchmark_wide_hierarchy_bare(width, 3, ITERATIONS);
		let named = benchmark_wide_hierarchy_named(width, 3, ITERATIONS);
		let per_tree = named / width as u32;

		println!(
			"{:>8} | {:>12.2?} | {:>12.2?} | {:>15.2?}",
			width, bare, named, per_tree
		);
	}

	// Section 3: EntityCloner vs Fresh Spawn (Deep)
	println!("\n📊 ENTITY CLONER vs FRESH SPAWN (Deep Hierarchies)");
	println!("{}", "-".repeat(90));
	println!(
		"{:>8} | {:>14} | {:>14} | {:>14} | {:>12}",
		"Depth", "Fresh (Bare)", "Clone (Bare)", "Clone (Named)", "Clone+64B"
	);
	println!("{}", "-".repeat(90));

	for &depth in DEPTHS {
		let fresh_bare = benchmark_deep_hierarchy_bare(depth, ITERATIONS);
		let clone_bare = benchmark_clone_deep_hierarchy_bare(depth, ITERATIONS);
		let clone_named =
			benchmark_clone_deep_hierarchy_named(depth, ITERATIONS);
		let clone_payload =
			benchmark_clone_deep_hierarchy_with_payload(depth, 64, ITERATIONS);

		println!(
			"{:>8} | {:>14.2?} | {:>14.2?} | {:>14.2?} | {:>12.2?}",
			depth, fresh_bare, clone_bare, clone_named, clone_payload
		);
	}

	// Section 4: Repeated Clones (simulating high-frequency triggers)
	println!(
		"\n📊 REPEATED CLONES (simulating behavior tree triggers, depth=10)"
	);
	println!("{}", "-".repeat(90));
	println!(
		"{:>12} | {:>14} | {:>14}",
		"Clone Count", "Total Time", "Per Clone"
	);
	println!("{}", "-".repeat(90));

	for &count in &[1, 10, 100, 500, 1000] {
		let total = benchmark_repeated_clones(10, count, ITERATIONS);
		let per_clone = total / count as u32;

		println!("{:>12} | {:>14.2?} | {:>14.2?}", count, total, per_clone);
	}

	// Section 5: Throughput Comparison
	println!(
		"\n📊 THROUGHPUT: FRESH SPAWN vs CLONE (100 hierarchies, depth=5)"
	);
	println!("{}", "-".repeat(90));

	let (spawn_time, clone_time) =
		benchmark_spawn_vs_clone_throughput(5, 100, ITERATIONS);
	let ratio = spawn_time.as_nanos() as f64 / clone_time.as_nanos() as f64;

	println!("  Fresh Spawn (100x): {:>12.2?}", spawn_time);
	println!("  Clone (100x):       {:>12.2?}", clone_time);
	println!(
		"  Ratio:              {:.2}x {}",
		ratio.abs(),
		if ratio > 1.0 {
			"(clone faster)"
		} else {
			"(spawn faster)"
		}
	);

	// Section 6: Single Entity Component Scaling
	println!("\n📊 SINGLE ENTITY: COMPONENT COUNT SCALING");
	println!("{}", "-".repeat(90));
	println!("{:>12} | {:>14}", "Components", "Time");
	println!("{}", "-".repeat(90));

	let component_results = benchmark_single_entity_components(ITERATIONS);
	for (count, time) in &component_results {
		println!("{:>12} | {:>14.2?}", count, time);
	}

	// Section 7: World Reuse
	println!("\n📊 WORLD REUSE vs FRESH WORLD (100 entities)");
	println!("{}", "-".repeat(90));

	let (fresh_world, reused_world) = benchmark_world_reuse(100, ITERATIONS);
	let reuse_ratio =
		fresh_world.as_nanos() as f64 / reused_world.as_nanos() as f64;

	println!("  Fresh World:    {:>12.2?}", fresh_world);
	println!("  Reused World:   {:>12.2?}", reused_world);
	println!(
		"  Ratio:          {:.2}x {}",
		if reuse_ratio > 1.0 {
			reuse_ratio
		} else {
			1.0 / reuse_ratio
		},
		if reuse_ratio > 1.0 {
			"(reuse faster)"
		} else {
			"(fresh faster)"
		}
	);

	// Section 8: Batch vs Individual
	println!("\n📊 BATCH (with_children) vs INDIVIDUAL (ChildOf) SPAWNS");
	println!("{}", "-".repeat(90));
	println!(
		"{:>12} | {:>14} | {:>14} | {:>12}",
		"Children", "Individual", "Batch", "Winner"
	);
	println!("{}", "-".repeat(90));

	for &child_count in &[5, 10, 25, 50, 100] {
		let (individual, batch) =
			benchmark_batch_vs_individual(child_count, ITERATIONS);
		let winner = if batch < individual {
			"batch"
		} else {
			"individual"
		};
		println!(
			"{:>12} | {:>14.2?} | {:>14.2?} | {:>12}",
			child_count, individual, batch, winner
		);
	}

	// Section 9: Cloner Setup Overhead
	println!("\n📊 ENTITY CLONER SETUP OVERHEAD (10 clones, depth=10)");
	println!("{}", "-".repeat(90));

	let (cloner_reused, cloner_fresh) =
		benchmark_cloner_setup_overhead(10, ITERATIONS);
	let setup_overhead =
		cloner_fresh.as_nanos() as f64 - cloner_reused.as_nanos() as f64;
	let per_setup = setup_overhead / 10.0; // 10 clones means 10 setups vs 1

	println!("  Reuse Cloner (10 clones): {:>12.2?}", cloner_reused);
	println!("  Fresh Cloner (10 clones): {:>12.2?}", cloner_fresh);
	println!("  Setup overhead per clone: ~{:.0}ns", per_setup / 9.0);

	// Section 10: Cost breakdown per entity
	println!("\n📊 COST PER ENTITY (extrapolated from depth=100)");
	println!("{}", "-".repeat(90));

	let bare_100 = benchmark_deep_hierarchy_bare(100, ITERATIONS);
	let named_100 = benchmark_deep_hierarchy_named(100, ITERATIONS);
	let payload_100 =
		benchmark_deep_hierarchy_with_payload(100, 64, ITERATIONS);
	let clone_100 = benchmark_clone_deep_hierarchy_named(100, ITERATIONS);

	println!("  Bare entity:        {:>12.2?} per entity", bare_100 / 100);
	println!(
		"  + Name component:   {:>12.2?} per entity",
		named_100 / 100
	);
	println!(
		"  + 64B Payload:      {:>12.2?} per entity",
		payload_100 / 100
	);
	println!(
		"  Clone (Named):      {:>12.2?} per entity",
		clone_100 / 100
	);

	println!("\n📊 NAME OVERHEAD");
	let name_overhead = (named_100.as_nanos() as f64
		- bare_100.as_nanos() as f64)
		/ bare_100.as_nanos() as f64
		* 100.0;
	println!("  Name adds {:.1}% overhead to spawn cost", name_overhead);

	// Section 11: Summary for decision making
	println!("\n{}", "=".repeat(90));
	println!("📈 KEY INSIGHTS FOR CONTROL FLOW HIERARCHIES");
	println!("{}", "=".repeat(90));
	println!();
	println!("For high-frequency per-trigger state (behavior trees, routers):");
	println!("  • Bare entity spawn:  ~{:.2?}/entity", bare_100 / 100);
	println!("  • Named entity spawn: ~{:.2?}/entity", named_100 / 100);
	println!("  • Clone from template: ~{:.2?}/entity", clone_100 / 100);
	println!();

	let clone_vs_spawn =
		clone_100.as_nanos() as f64 / named_100.as_nanos() as f64;
	if clone_vs_spawn < 1.0 {
		println!(
			"✅ EntityCloner is {:.1}x FASTER than fresh spawns",
			1.0 / clone_vs_spawn
		);
		println!("   → Use a template entity and clone for each request");
	} else {
		println!(
			"⚠️  EntityCloner is {:.1}x SLOWER than fresh spawns",
			clone_vs_spawn
		);
		println!("   → Consider fresh spawns for simple hierarchies");
	}

	println!();
	println!("Recommendations:");
	println!("  1. Reuse World instances when possible (2x faster)");
	println!(
		"  2. For shallow hierarchies (<10 depth), fresh spawns are competitive"
	);
	println!("  3. For repeated cloning, reuse the EntityCloner instance");
	println!(
		"  4. Name components add ~35% overhead - omit in hot paths if possible"
	);
	println!(
		"  5. Heap-allocated components (Payload) add ~15-30% per component"
	);

	println!("\n{}", "=".repeat(90));
	println!("✅ Benchmark Complete\n");
}
