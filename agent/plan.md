beet_core no_std

Beet core should be no_std with no-default-features, and also with as many features as possible.

Many crates support no-std, ie serde with no-default-features. lean into this where possible.

Here are some common patterns for toggling std. Its up to you which one to choose, but either way we do not want to be importing `alloc` everywhere in the crate. In other words, whether or not we are `std` should generally only be dealt with at the `lib.rs` level, not splattered throughout the crate.

if need be re-export core and alloc stuff in the prelude.

```rust
#![no_std]
// alternatively
#![cfg_attr(not(feature = "std"), no_std)]

// adding std back in
#[cfg(feature = "std")]
extern crate std;

// feature gated?
extern crate alloc;


// example of reexporting so we dont need to dance with core & alloc all day
pub mod prelude{
	pub(crate) use std_export::*;
}

mod std_export{
	pub use core::*;
	pub use core::boxed::Box;
	pub use alloc::*;
	pub use alloc::vec::Vec;
	
}

```


## Success

verify success by ensuring the following works:
`cargo test -p beet_core`
`cargo test -p beet_core --all-features`
`cargo test -p beet_core --no-default-features`
and perhaps testing some of the more nuanced features individually too.
use tail when calling these commands to avoid context bloat.
