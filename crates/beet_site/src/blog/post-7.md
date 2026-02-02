+++
title="The Harvest #7"
created="2026-01-04"
+++

# The Harvest #7 - Malleable Application Framework

<iframe src="https://www.youtube.com/embed/ycOUd6f0XRw" title="Full Moon Harvest #7 | Malleable Application Framework" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>

<br/>
<br/>

This post and [pr #75](https://github.com/mrchantey/beet/pull/75) marks the completion of the great ECS-ification. The test runner was the final crate built in the hap-hazard 'c++ style oop' and now that it has been converted the beet repo is entirely ECS.

In celebration I'd like to dive a bit deeper into this specific refactor. Its all well and good to make grandious statements how ECS architecture is the bees knees, but these kinds of claims are best served alongside concrete examples.

### Before: Seperate Runtimes

The very first thing the original runner does is fork behavior between native and wasm. These two environments are very different, for example a wasm app must first yield to the event loop in order to run async tests, and accomodating for this resulted in two completely different runtimes.

```rust
pub fn test_runner(tests: &[&TestDescAndFn]) {	
	#[cfg(target_arch = "wasm32")]
	test_runner_wasm(tests);
	#[cfg(not(target_arch = "wasm32"))]
	test_runner_native(tests);
}
```

### After: Unified Runtime

Bevy has already solved a lot of the cross-platform differences for us, now the test runner is a regular bevy app. This is a shift from a unique and and bespoke implementation, to a standardized well trodden path, reducing bugs and maintenance costs.

```rust
pub fn test_runner(tests: &[&TestDescAndFn]) {
	App::new()
		.add_plugins(TestPlugin)
		.spawn_then(tests_bundle(tests))
		.run();
}
```


### Before: Abstract Traits

Extensibility was always a goal for sweet, the original implementation was full of traits, best-effort attempts to accomodate for current and future needs. A lot of busy-work is involved in moving these strategy types around, and the traits inevitably become tangled as requirements shift.

```rust
struct TestHarness {
	suites: Vec<TestSuite>,
	runner: Box<dyn Runner>,
	case_logger: Box<dyn CaseLogger>,
	suite_logger: Box<dyn SuiteLogger>,
}

struct TestSuite {
	cases: Vec<TestCase>,
	runner: Box<dyn Runner>,
	case_logger: Box<dyn CaseLogger>,
	suite_logger: Box<dyn SuiteLogger>,
}
```


### After: Flat System Architecture

The flat architecture of ECS systems is much easier to reason about. For instance the `log_case_outcomes` system is right there in plain sight, no longer buried under layers of traits and indirection. This also trivializes modding, to use a parallel runner we'd just replace the `run_tests_series` system.

```rust
impl Plugin for TestPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
				Update,
				(
					log_suite_running,
					filter_tests,
					log_case_running,
					(run_tests_series, run_non_send_tests_series),
					trigger_timeouts,
					insert_suite_outcome,
					log_case_outcomes,
					log_suite_outcome,
					exit_on_suite_outcome,
				)
					.chain(),
			);
	}
}
```

## Branding

In other news the repo has had a slight adjustment from last months rebranding:

> A Malleable Application Framework

Mostly inspired by some recent reading I've been doing:

## CodingItWrong - User-modifiable software
- [Essay](https://usermodifiable.codingitwrong.com/)
- [YouTube](https://youtu.be/x-FkNd5DkOQ?si=yXyLtS1k0Cs5TpKO)

This is a great breakdown on two pioneers in malleable applications: `smalltalk` and `hypercard`.

## InkAndSwitch - Malleable Software
- [Essay](https://www.inkandswitch.com/essay/malleable-software/)

The phrasing used in this essay is really growing on me. The term 'malleable' has this tangible quality I really like, and reflects well this idea of technology that can be bent into shape without breaking.

## Maggie Appleton: Bare-foot developers
- [Blog Post](https://maggieappleton.com/home-cooked-software)
- [YouTube](https://youtu.be/qo5m92-9_QI)

I think Maggie's concept of 'bare-foot developers' well articulates the semi-technical as a key audience for malleable applications.
