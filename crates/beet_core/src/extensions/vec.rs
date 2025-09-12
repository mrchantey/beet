pub struct VecExt<T> {
	phantom: std::marker::PhantomData<T>,
}

impl<T> VecExt<T> {
	pub fn entry_or_insert_with(
		vec: &mut Vec<T>,
		predicate: impl Fn(&T) -> bool,
		default: impl FnOnce() -> T,
	) -> &mut T {
		for i in 0..vec.len() {
			if predicate(&vec[i]) {
				return &mut vec[i];
			}
		}
		vec.push(default());
		vec.last_mut().unwrap()
	}
}
