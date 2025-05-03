---
title: Matchers
description: Introduction to sweet matchers
draft: true
sidebar:
  order: 2
---



Matchers are an ergonomic and performant way to make type-specific assertions, outperforming `assert!` [by 100x](./assert.md).

```rs
expect(true).to_be_true();
```

All matchers can be negated by calling `not()`.

```rs
expect("foobar").not().to_contain("bazz");
```

## Built-in Matchers

```rs
expect("foobar").to_start_with("foo");
expect(my_result).to_be_ok();
expect(2).to_be_greater_than(1);
```

## Extending Matchers

Matchers are easy to extend, particulary using the `extend` crate.

```rust
use anyhow::Result;
use extend::ext;
use sweet::prelude::*;

#[derive(Debug)]
struct Awesomeness(u32);

#[ext]
pub impl Matcher<Awesomeness> {
	fn to_be_more_awesome_than(&self, other:Awesomeness) {
		let outcome = self.0 > other.0;
		let expected = format!("to be more awesome than {:?}", other);
		self.assert_correct(outcome, &expected);
	}
}
```

Note that here we are calling `self.assert_correct()` which does two things:
- Handles negation: `not()`
- Unwinds the backtrace output to one level up by emitting a specially formatted panic.


Its important that assert_correct is called in the matcher and not an extra layer deeper. This would result in a backtrace showing `self.sub_func()` instead of the `expect()` location.

```diff lang="rust" title="Incorrect"
fn to_show_a_correct_backtrace(&self) {
-	self.sub_func();
}
fn sub_func(&self){
-	self.assert_correct(false, "");
}

```
