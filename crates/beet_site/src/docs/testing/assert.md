---
title: On Assert
description: The dark side of assert
draft: true
sidebar:
  order: 99
---

First of all let me assert that `assert!` does have a place, but its absolutely not in tests. The TLDR is that it provides a simple way to collect error locations but strikes at rust's achilles heel, *compile times*, and the same information can be achieve lazily through runtime backtracing.

## The Bench

- [source code](https://github.com/mrchantey/sweet/blob/main/cli/src/bench/bench_assert.rs)
- have a play `cargo install sweet-cli && sweet bench-assert --iterations 2000`


Its common knowledge that even simple macros increase compile times, but did you ever wonder how much? The answer turns out to be a few milliseconds. The below benches were created by generating files with `n` lines of either `assert!` or a wrapper `expect` function.


_I dont consider myself a benching wizard, if you see a way this approach could be improved please [file an issue](https://github.com/mrchantey/sweet/issues) or pr. I'm particularly curious about what happened at the 20,000 line mark._

### Implications:

For some real world context, here's some 'back of a napkin' calculations i did by grepping a few rust repos i had laying around:
| Repo         | `assert!` Lines [^3] | `assert!` Compile Time | `expect` Compile Time |
| ------------ | -------------------- | ---------------------- | --------------------- |
| bevy         | 7,000                | 30s                    | 0.3s                   |
| wasm-bindgen | 3,000                | 15s                    | 0.15s                  |
| rust         | 50,000               | 250s                   | 2.5s                  |

[^3]: A very coarse grep of `assert!` or `assert_`

### Assert: `5ms`

Creating a file with `n` number of lines with an `assert_eq!(n,n)`, calcualting how long it takes to compile the assert! macro.

| Lines  | Compilation [^1] | Time per Line [^2] | Notes                                       |
| ------ | ---------------- | ------------------ | ------------------------------------------- |
| 10     | 0.21s            | 21.00ms            |                                             |
| 100    | 0.23s            | 2.30ms             |                                             |
| 1,000  | 1.54s            | 1.54ms             |                                             |
| 2,000  | 4.92s            | 2.46ms             |                                             |
| 3,000  | 11.61s           | 3.87ms             |                                             |
| 5,000  | 26.96s           | 5.39ms             |                                             |
| 10,000 | 55.00s           | 5.50ms             |                                             |
| 20,000 | 1.06s            | 0.05ms             | this is incorrect, it actually took 10 mins |


### Expect: `0.05ms`

Creating a file with `n` number of lines with an assert! wrapper function called `expect(n,n)`. This bench essentially calculates how long it takes to compile the calling of a regular rust function.

| Lines   | Compilation [^1] | Time per Line [^2] |
| ------- | ---------------- | ------------------ |
| 10      | 0.53s            | 53.00ms            |
| 100     | 0.47s            | 4.70ms             |
| 1,000   | 0.49s            | 0.49ms             |
| 2,000   | 0.50s            | 0.25ms             |
| 3,000   | 0.53s            | 0.18ms             |
| 5,000   | 0.56s            | 0.11ms             |
| 10,000  | 0.70s            | 0.07ms             |
| 20,000  | 1.06s            | 0.05ms             |
| 100,000 | 5.37s            | 0.05ms             |
| 500,000 | 44.**00s**           | 0.09ms             |

[^1]: Compile times are retrieved from the output of `cargo build`, `Finished dev [unoptimized + debuginfo] target(s) in 0.33 secs`
[^2]: Time per line is simply ` line count / compile time`

## The Alternative - Matchers

The alternative requires both the matcher and the runner to work in unison with three rules:

1. The `expect()` function must panic exactly one frame beneath the caller and always outputs some prefix in the payload string, in sweet this is `"Sweet Error:"`
2. If the runner encounters a regular panic, just use the panics location for pretty printing.
3. If the runner encounters a panic with the prefix, create a backtracer and use the location exactly one frame up the callstack.