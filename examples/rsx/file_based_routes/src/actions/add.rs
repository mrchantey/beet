use beet::prelude::*;




pub fn get(In((a, b)): In<(u32, u32)>) -> u32 { a + b }
