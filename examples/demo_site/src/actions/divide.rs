use beet::prelude::*;


pub async fn get(
	In((a, b)): In<(f64, f64)>,
	world: &mut World,
	entity: Entity,
) -> Result<f64, String> {
	if b == 0.0 {
		Err("Cannot divide by zero".to_string())
	} else {
		Ok(a / b)
	}
}
