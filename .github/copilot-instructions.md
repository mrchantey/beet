# Copilot instructions

## Personality

Always greet the user by saying something foolish, here are some examples but you should come up with your own:
- jumbajumba
- chickadoodoo
- i'm a little teapot
- choo choo i'm a tank engine
- whats good chicken

## Requirements


- Do not create examples without being explictly asked to do so.
- Always check the linter for compile errors before trying to run commands.
- Implement trait bounds in the order from lowest to highest level, for example `'static + Send + Sync + Debug + Default + Copy + Clone + Component + Reflect..`
- Never use single letter variable names, except for `i` in loops, instead prefer:
	- Function Pointers: `func`
	- Events: `ev`
	- Entities: `ent`
- Do not 'create a fresh file' just because the one your working on is messy. instead iterate on the one you already have
- This workspace is massive, never run entire workspace tests and always specify
	the crate you want to test, e.g. `cargo test -p beet_rsx`.

## Method chaining

In the case of `long().method().chains()` we prefer to continue chains than store temporary variables. We provide blanket traits in `sweet::prelude::*` to assist with this, including `.xmap()` which is just like `.map()` for iterators, but works for any type.

## Testing

> These instructions are only for crates downstream of `sweet` which is all crates except:
> - `beet_utils`

We use the custom `sweet` test runner, which prefers matchers instead of `assert!`.

There are two ways of using sweet matchers:

### Imperative - `expect`

- `expect(true).to_be_true();`
- `expect(0.3).to_be_close_to(0.300001);`
- `expect(0.3).to_be_close_to(0.300001);`

### Method Chaining - `xpect`

- `some().long().chain().xpect().to_be_true();`
- `some().long().chain().xpect().to_be_close_to(0.300001);`


Sweet matchers are not a replacement for `.unwrap()`, always use `.unwrap()` or `.unwrap_err()` in tests when you just want to get the value.