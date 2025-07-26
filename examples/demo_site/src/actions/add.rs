use beet::prelude::*;




pub fn get(In((a, b)): In<(i32, i32)>) -> i32 { a + b }
