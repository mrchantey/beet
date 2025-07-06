/// This macro is transparent, it returns the expression as is.
/// Required so we can efficiently only visit macros, instead of
/// checking every expression. Means we can only replace the macro, not delete it.
#[macro_export]
macro_rules! noop {
	($($t:tt)*) => {
		$($t)*
	};
}
