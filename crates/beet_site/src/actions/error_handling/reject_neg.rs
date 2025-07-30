use beet::prelude::*;



/// rejects any negative number
pub fn post(In(a): In<i32>) -> Result<u32, String> {
	if a >= 0 {
		Ok(a as u32)
	} else {
		Err(format!("expected positive number, received {a}"))
	}
}
