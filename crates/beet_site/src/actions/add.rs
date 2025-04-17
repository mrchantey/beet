use axum::prelude::*;


pub fn get(JsonQuery((a, b)): JsonQuery<(i32, i32)>) -> Json<i32> {
	Json(a + b)
}
