# Sweet Test

A pretty cross platform test runner.

# Usage

```rust
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet_test::test_runner))]
use sweet_test::prelude::*;

#[test]
fn it_passes(){
	assert!(1 + 1 == 2);
	expect("sweet").not().to_contain("verbose matchers");
}

```

```sh
cargo test
```