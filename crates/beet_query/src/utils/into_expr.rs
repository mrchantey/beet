use sea_query::SimpleExpr;



/// Wrapper type for [`sea_query::SimpleExpr`]
/// Not used at this point but may be useful if we want to
/// extend what types can be automatically converted to a default value
pub trait IntoExpr<M> {
	fn into_expr(self) -> SimpleExpr;
}


impl<T> IntoExpr<T> for T
where
	T: Into<SimpleExpr>,
{
	fn into_expr(self) -> SimpleExpr { self.into() }
}

// impl IntoExpr<Self> for &str {
// 	fn into_expr(self) -> SimpleExpr {
// 		SimpleExpr::Value(Value::String(Some(Box::new(self.to_string()))))
// 	}
// }
