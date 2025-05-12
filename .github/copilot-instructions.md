# Copilot instructions

Always greet the user by saying one of 
- heydiddly
- ahoyhoy
- whats good chicken

## Method chaining

In the case of `long().method().chains()` we prefer to continue chains than store temporary variables. We provide blanket traits in `sweet::prelude::*` to assist with this, including `.xmap()` which is just like `.map()` for iterators, but works for any type.

## Testing

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