# Contributing

👋 Thanks for your interest in beet! 
My name's Pete (@mrchantey) and I'm working on this full-time. Come and say hi in the [bevy/ecosystem-crates/beet](https://discord.com/channels/691052431525675048/1333204907414523964) discord channel!

## Conventions

Bevy has built a fantastic system for ambitious framework development, and we can ride in the slipstream of big bird by following its lead in every way, with exceptions where it makes sense.


### Bevy Conventions

- community structure, I think this is bevy's number one feature
- project structure
- linting, formatting, clippy, ci, etc
- clean apis, using macros sparingly

### Beet Conventions


#### Testing

I've built a test runner, `sweet`, which is a superset of the default runner, meaning any default cli command, ie `cargo test --test-threads=4`, or testing convention, ie `#[tokio::test]` and `assert!()`, is supported. 
In addition it incudes tools required for the kind of work we're doing at this higher level of the stack:
- Pretty and clickable test outputs
- running `#[test]` in wasm
- Matchers, ie `expect(true).to_be_true()`

This is still a work in progress and has sharp edges, but I think worth taking the time to get right.

### Current State

The above is the goal but not the current state of the project for a few reasons.

1. I'm still learning rust conventions, having come from a web/unity background. For example I'm more used to treating a `mod.rs` like an `index.ts`, whose sole purpose is to re-export other files in the dir.
2. Beet is a construction site, it has many moving parts and getting the abstraction layers right is a wip.

To be clear these are habits not opinions, I'm very happy to discuss and change any convention where doing so would improve the productivity of the project.