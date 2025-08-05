use beet::prelude::*;


pub fn get(In((a, b)): In<(f64, f64)>) -> Result<f64, String> {
	if b == 0.0 {
		Err("Cannot divide by zero".to_string())
	} else {
		Ok(a / b)
	}
}
