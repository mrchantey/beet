//! # Results
//! 
//! `cargo run --release --bin bevy_app_coldstart`
//! 
//! ## Without `bevy/multi_threaded` feature enabled
//! 
//! 📊 BENCHMARK RESULTS (Average time per operation)
//! ============================================================
//! 
//! 🔄 Simple For Loop:
//!   Average: 1.625µs
//!   Min:     1.408µs
//!   Max:     1.95µs
//! 
//! � World Only (run_system_cached):
//!   Average: 8.69µs
//!   Min:     7.703µs
//!   Max:     9.224µs
//! 
//! 📦 App (World Only):
//!   Average: 29.708µs
//!   Min:     27.955µs
//!   Max:     31.03µs
//! 
//! 📦 App (No Plugins):
//!   Average: 52.072µs
//!   Min:     46.22µs
//!   Max:     56.235µs
//! 
//! ⚡ App (Minimal Plugins):
//!   Average: 121.789µs
//!   Min:     112.131µs
//!   Max:     132.541µs
//! 
//! 🎮 App (Default Plugins):
//!   Average: 306.736µs
//!   Min:     280.145µs
//!   Max:     320.972µs
//! 
//! ⚡ Performance Comparison (relative to simple loop):
//!   World Only:      5.3x slower
//!   App World Only:  18.3x slower
//!   App No Plugins:  32.0x slower
//!   App Minimal:     74.9x slower
//!   App Default:     188.8x slower
//! 
//! 📈 Overhead Progression:
//!   Simple For Loop → World Only:    5.3x increase
//!   World Only → App World Only:     3.4x increase
//!   App World Only → App No Plugins: 1.8x increase
//!   App No Plugins → Minimal:        2.3x increase
//!   Minimal → Default Plugins:       2.5x increase


//! ## With `bevy/multi_threaded` feature enabled
//!
//! 📊 BENCHMARK RESULTS (Average time per operation)
//! ============================================================
//! 
//! 🔄 Simple For Loop:
//!   Average: 1.589µs
//!   Min:     1.408µs
//!   Max:     1.842µs
//! 
//! � World Only (run_system_cached):
//!   Average: 11.816µs
//!   Min:     9.868µs
//!   Max:     14.398µs
//! 
//! 📦 App (World Only):
//!   Average: 30.959µs
//!   Min:     29.301µs
//!   Max:     34.023µs
//! 
//! 📦 App (No Plugins):
//!   Average: 611.356µs
//!   Min:     591.509µs
//!   Max:     645.219µs
//! 
//! ⚡ App (Minimal Plugins):
//!   Average: 1.275514ms
//!   Min:     1.230887ms
//!   Max:     1.322509ms
//! 
//! 🎮 App (Default Plugins):
//!   Average: 2.85952ms
//!   Min:     2.786892ms
//!   Max:     2.908475ms
//! ⚡ Performance Comparison (relative to simple loop):
//!   World Only:      7.4x slower
//!   App World Only:  19.5x slower
//!   App No Plugins:  384.7x slower
//!   App Minimal:     802.7x slower
//!   App Default:     1799.6x slower
//! 
//! 📈 Overhead Progression:
//!   Simple For Loop → World Only:    7.4x increase
//!   World Only → App World Only:     2.6x increase
//!   App World Only → App No Plugins: 19.7x increase
//!   App No Plugins → Minimal:        2.1x increase
//!   Minimal → Default Plugins:       2.2x increase

use bevy::prelude::*;
use std::time::Duration;
use std::time::Instant;


#[derive(Component)]
struct Counter {
	value: usize,
}

#[derive(Resource)]
struct BenchmarkConfig {
	entity_operations: usize,
	current_count: usize,
}

fn setup_counter_system(mut commands: Commands) {
	commands.spawn(Counter { value: 0 });
}

fn increment_counter_system(
	mut query: Query<&mut Counter>,
	mut config: ResMut<BenchmarkConfig>,
) {
	if let Ok(mut counter) = query.single_mut() {
		counter.value += 1;
		config.current_count += 1;
	}
}

fn print_final_count_system(query: Query<&Counter>) {
	if let Ok(counter) = query.single() {
		println!("Final Bevy counter value: {}", counter.value);
	}
}

fn run_world_only_benchmark(num_operations: usize) -> Duration {
	let mut individual_times = Vec::new();

	// For each operation, measure the time to create a new world and run systems directly
	for _ in 0..num_operations {
		let start_time = Instant::now();
		
		let mut world = World::new();
		world.insert_resource(BenchmarkConfig {
			entity_operations: 1,
			current_count: 0,
		});

		// Run setup system
		world.run_system_cached(setup_counter_system);
		// Run increment system
		world.run_system_cached(increment_counter_system);

		individual_times.push(start_time.elapsed());
	}

	// Return average time per operation
	individual_times.iter().sum::<Duration>() / num_operations as u32
}

fn run_app_no_plugins_benchmark(num_operations: usize) -> Duration {
	let mut individual_times = Vec::new();

	// For each operation, measure the time to spin up a new Bevy app with no plugins
	for _ in 0..num_operations {
		let start_time = Instant::now();
		
		let mut app = App::new();
		app
			.insert_resource(BenchmarkConfig {
				entity_operations: 1,
				current_count: 0,
			})
			.add_systems(Startup, setup_counter_system)
			.add_systems(Update, increment_counter_system);

		// Run the app once to do the single operation
		app.update(); // startup
		app.update(); // single increment

		individual_times.push(start_time.elapsed());
	}

	// Return average time per operation
	individual_times.iter().sum::<Duration>() / num_operations as u32
}

fn run_app_minimal_plugins_benchmark(num_operations: usize) -> Duration {
	let mut individual_times = Vec::new();

	// For each operation, measure the time to spin up a new Bevy app with minimal plugins
	for _ in 0..num_operations {
		let start_time = Instant::now();
		
		let mut app = App::new();
		app
			.add_plugins(MinimalPlugins)
			.insert_resource(BenchmarkConfig {
				entity_operations: 1,
				current_count: 0,
			})
			.add_systems(Startup, setup_counter_system)
			.add_systems(Update, increment_counter_system);

		// Run the app once to do the single operation
		app.update(); // startup
		app.update(); // single increment

		individual_times.push(start_time.elapsed());
	}

	// Return average time per operation
	individual_times.iter().sum::<Duration>() / num_operations as u32
}

fn run_app_default_plugins_benchmark(num_operations: usize) -> Duration {
	let mut individual_times = Vec::new();

	// For each operation, measure the time to spin up a new Bevy app with default plugins
	for _ in 0..num_operations {
		let start_time = Instant::now();
		
		let mut app = App::new();
		app
			.add_plugins(DefaultPlugins.build().disable::<bevy::log::LogPlugin>())
			.insert_resource(BenchmarkConfig {
				entity_operations: 1,
				current_count: 0,
			})
			.add_systems(Startup, setup_counter_system)
			.add_systems(Update, increment_counter_system);

		// Run the app once to do the single operation
		app.update(); // startup
		app.update(); // single increment

		individual_times.push(start_time.elapsed());
	}

	// Return average time per operation
	individual_times.iter().sum::<Duration>() / num_operations as u32
}

fn run_app_world_only_benchmark(num_operations: usize) -> Duration {
	let mut individual_times = Vec::new();

	// For each operation, measure the time to create a new app but only use its world
	for _ in 0..num_operations {
		let start_time = Instant::now();
		
		let mut app = App::new();
		app.insert_resource(BenchmarkConfig {
			entity_operations: 1,
			current_count: 0,
		});

		// Run systems directly on the world without using app.update()
		app.world_mut().run_system_cached(setup_counter_system);
		app.world_mut().run_system_cached(increment_counter_system);

		individual_times.push(start_time.elapsed());
	}

	// Return average time per operation
	individual_times.iter().sum::<Duration>() / num_operations as u32
}

fn run_simple_loop_benchmark(iterations: usize) -> Duration {
	let start_time = Instant::now();

	let mut counter = 0;
	for _ in 0..iterations {
		counter += 1;
	}

	println!("Final simple loop counter value: {}", counter);
	start_time.elapsed()
}

fn main() {
	const NUM_OPERATIONS: usize = 500; // Reduced for cold start testing
	const TEST_ITERATIONS: usize = 3; // Multiple iterations for accuracy

	println!("🚀 Bevy App Cold Start Benchmark");
	println!("Operations per test: {}", NUM_OPERATIONS);
	println!("Test iterations: {}", TEST_ITERATIONS);
	println!("{}", "=".repeat(60));

	// Scenario 1: World only with run_system_cached
	let mut world_times: Vec<Duration> = Vec::new();
	println!("Running World-only benchmarks (run_system_cached)...");
	for iteration in 1..=TEST_ITERATIONS {
		println!("World iteration {}/{}", iteration, TEST_ITERATIONS);
		let duration = run_world_only_benchmark(NUM_OPERATIONS);
		world_times.push(duration);
		println!("  Time: {:?}", duration);
	}

	// Scenario 2: App but only use world
	let mut app_world_only_times: Vec<Duration> = Vec::new();
	println!("\nRunning App (world only) benchmarks...");
	for iteration in 1..=TEST_ITERATIONS {
		println!("App world only iteration {}/{}", iteration, TEST_ITERATIONS);
		let duration = run_app_world_only_benchmark(NUM_OPERATIONS);
		app_world_only_times.push(duration);
		println!("  Time: {:?}", duration);
	}

	// Scenario 3: App with no plugins
	let mut app_no_plugins_times: Vec<Duration> = Vec::new();
	println!("\nRunning App (no plugins) benchmarks...");
	for iteration in 1..=TEST_ITERATIONS {
		println!("App no plugins iteration {}/{}", iteration, TEST_ITERATIONS);
		let duration = run_app_no_plugins_benchmark(NUM_OPERATIONS);
		app_no_plugins_times.push(duration);
		println!("  Time: {:?}", duration);
	}

	// Scenario 4: App with minimal plugins
	let mut app_minimal_plugins_times: Vec<Duration> = Vec::new();
	println!("\nRunning App (minimal plugins) benchmarks...");
	for iteration in 1..=TEST_ITERATIONS {
		println!("App minimal plugins iteration {}/{}", iteration, TEST_ITERATIONS);
		let duration = run_app_minimal_plugins_benchmark(NUM_OPERATIONS);
		app_minimal_plugins_times.push(duration);
		println!("  Time: {:?}", duration);
	}

	// Scenario 5: App with default plugins
	let mut app_default_plugins_times: Vec<Duration> = Vec::new();
	println!("\nRunning App (default plugins) benchmarks...");
	for iteration in 1..=TEST_ITERATIONS {
		println!("App default plugins iteration {}/{}", iteration, TEST_ITERATIONS);
		let duration = run_app_default_plugins_benchmark(NUM_OPERATIONS);
		app_default_plugins_times.push(duration);
		println!("  Time: {:?}", duration);
	}

	// Scenario 5: App but only using world
	let mut app_world_only_times: Vec<Duration> = Vec::new();
	println!("\nRunning App (world only) benchmarks...");
	for iteration in 1..=TEST_ITERATIONS {
		println!("App world only iteration {}/{}", iteration, TEST_ITERATIONS);
		let duration = run_app_world_only_benchmark(NUM_OPERATIONS);
		app_world_only_times.push(duration);
		println!("  Time: {:?}", duration);
	}

	// Simple loop benchmark for comparison
	let mut loop_times: Vec<Duration> = Vec::new();
	println!("\nRunning simple loop benchmarks...");
	for iteration in 1..=TEST_ITERATIONS {
		println!("Loop iteration {}/{}", iteration, TEST_ITERATIONS);
		let duration = run_simple_loop_benchmark(NUM_OPERATIONS);
		loop_times.push(duration);
		println!("  Time: {:?}", duration);
	}

	// Calculate averages
	let world_avg = world_times.iter().sum::<Duration>() / world_times.len() as u32;
	let app_no_plugins_avg = app_no_plugins_times.iter().sum::<Duration>() / app_no_plugins_times.len() as u32;
	let app_minimal_plugins_avg = app_minimal_plugins_times.iter().sum::<Duration>() / app_minimal_plugins_times.len() as u32;
	let app_default_plugins_avg = app_default_plugins_times.iter().sum::<Duration>() / app_default_plugins_times.len() as u32;
	let app_world_only_avg = app_world_only_times.iter().sum::<Duration>() / app_world_only_times.len() as u32;
	let loop_avg = loop_times.iter().sum::<Duration>() / loop_times.len() as u32;

	// Results
	println!("\n\n");
	println!("📊 BENCHMARK RESULTS (Average time per operation)");
	println!("{}", "=".repeat(60));

	println!("\n🔄 Simple For Loop:");
	println!("  Average: {:?}", loop_avg);
	println!("  Min:     {:?}", loop_times.iter().min().unwrap());
	println!("  Max:     {:?}", loop_times.iter().max().unwrap());

	println!("\n� World Only (run_system_cached):");
	println!("  Average: {:?}", world_avg);
	println!("  Min:     {:?}", world_times.iter().min().unwrap());
	println!("  Max:     {:?}", world_times.iter().max().unwrap());

	println!("\n📦 App (World Only):");
	println!("  Average: {:?}", app_world_only_avg);
	println!("  Min:     {:?}", app_world_only_times.iter().min().unwrap());
	println!("  Max:     {:?}", app_world_only_times.iter().max().unwrap());

	println!("\n📦 App (No Plugins):");
	println!("  Average: {:?}", app_no_plugins_avg);
	println!("  Min:     {:?}", app_no_plugins_times.iter().min().unwrap());
	println!("  Max:     {:?}", app_no_plugins_times.iter().max().unwrap());

	println!("\n⚡ App (Minimal Plugins):");
	println!("  Average: {:?}", app_minimal_plugins_avg);
	println!("  Min:     {:?}", app_minimal_plugins_times.iter().min().unwrap());
	println!("  Max:     {:?}", app_minimal_plugins_times.iter().max().unwrap());

	println!("\n🎮 App (Default Plugins):");
	println!("  Average: {:?}", app_default_plugins_avg);
	println!("  Min:     {:?}", app_default_plugins_times.iter().min().unwrap());
	println!("  Max:     {:?}", app_default_plugins_times.iter().max().unwrap());


	// Performance comparisons
	println!("\n⚡ Performance Comparison (relative to simple loop):");
	let world_ratio = world_avg.as_nanos() as f64 / loop_avg.as_nanos() as f64;
	let app_world_only_ratio = app_world_only_avg.as_nanos() as f64 / loop_avg.as_nanos() as f64;
	let app_no_plugins_ratio = app_no_plugins_avg.as_nanos() as f64 / loop_avg.as_nanos() as f64;
	let app_minimal_plugins_ratio = app_minimal_plugins_avg.as_nanos() as f64 / loop_avg.as_nanos() as f64;
	let app_default_plugins_ratio = app_default_plugins_avg.as_nanos() as f64 / loop_avg.as_nanos() as f64;
	let app_world_only_ratio = app_world_only_avg.as_nanos() as f64 / loop_avg.as_nanos() as f64;

	println!("  World Only:      {:.1}x slower", world_ratio);
	println!("  App World Only:  {:.1}x slower", app_world_only_ratio);
	println!("  App No Plugins:  {:.1}x slower", app_no_plugins_ratio);
	println!("  App Minimal:     {:.1}x slower", app_minimal_plugins_ratio);
	println!("  App Default:     {:.1}x slower", app_default_plugins_ratio);

	println!("\n📈 Overhead Progression:");
	println!("  Simple For Loop → World Only:    {:.1}x increase", world_ratio);
	println!("  World Only → App World Only:     {:.1}x increase", app_world_only_ratio / world_ratio);
	println!("  App World Only → App No Plugins: {:.1}x increase", app_no_plugins_ratio / app_world_only_ratio);
	println!("  App No Plugins → Minimal:        {:.1}x increase", app_minimal_plugins_ratio / app_no_plugins_ratio);
	println!("  Minimal → Default Plugins:       {:.1}x increase", app_default_plugins_ratio / app_minimal_plugins_ratio);
}
