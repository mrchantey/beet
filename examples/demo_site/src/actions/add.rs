use beet::prelude::*;




pub fn get(In(a): In<i32>) -> i32 { a + 1 }
