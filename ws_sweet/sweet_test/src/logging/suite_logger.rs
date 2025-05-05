pub trait SuiteLogger
where
	Self: Sized,
{
	fn on_start(start_str: String) -> Self;
	fn on_end(self, end_str: String);
}
