/// Convenience trait so the same matchers and assertions can be used
/// across types, for instance comparing `T` with `&T`.
pub trait SweetRef<T> {
	fn sweet_ref(&self) -> &T;
}
pub trait SweetMut<T>: SweetRef<T> {
	fn sweet_mut(&mut self) -> &mut T;
}

/// Implements [`SweetRef`] and [`SweetMut`] for the given type.
///
/// ```
/// # use sweet_test::prelude::*;
/// struct MyStruct;
/// sweet_ref_impls!(MyStruct);
///
/// struct MyGenericStruct<T>(T);
/// sweet_ref_impls!(MyGenericStruct; T);
/// ```
#[macro_export]
macro_rules! sweet_ref_impls {
    // Non-generic type, default to self
    ($ty:ty) => {
        sweet_ref_impls!($ty; |s| s; |s| s);
    };
    // Non-generic type, custom getters
    (
			$ty:ty;
			$get_ref:expr;
			$get_mut:expr
		) => {
        impl SweetRef<$ty> for $ty {
            fn sweet_ref(&self) -> &$ty { $get_ref(self) }
        }
        impl SweetMut<$ty> for $ty {
            fn sweet_mut(&mut self) -> &mut $ty { $get_mut(self) }
        }
        impl SweetRef<$ty> for &$ty {
            fn sweet_ref(&self) -> &$ty { $get_ref(self) }
        }
        impl SweetRef<$ty> for &mut $ty {
            fn sweet_ref(&self) -> &$ty { $get_ref(self) }
        }
        impl SweetMut<$ty> for &mut $ty {
            fn sweet_mut(&mut self) -> &mut $ty { $get_mut(self) }
        }
    };
    // Generic type, default to self
    (
			$ty:ident;
			$($gen:ident),+
		) => {
        sweet_ref_impls!($ty;$($gen),+; |s| s; |s| s);
    };
    // Generic type, custom getters
    (
			$ty:ident;
			$($gen:ident),+;
			$get_ref:expr;
			$get_mut:expr
		) => {
        impl<$($gen),+> SweetRef<$ty<$($gen),+>> for $ty<$($gen),+> {
            fn sweet_ref(&self) -> &$ty<$($gen),+> { $get_ref(self) }
        }
        impl<$($gen),+> SweetMut<$ty<$($gen),+>> for $ty<$($gen),+> {
            fn sweet_mut(&mut self) -> &mut $ty<$($gen),+> { $get_mut(self) }
        }
        impl<$($gen),+> SweetRef<$ty<$($gen),+>> for &$ty<$($gen),+> {
            fn sweet_ref(&self) -> &$ty<$($gen),+> { $get_ref(self) }
        }
        impl<$($gen),+> SweetRef<$ty<$($gen),+>> for &mut $ty<$($gen),+> {
            fn sweet_ref(&self) -> &$ty<$($gen),+> { $get_ref(self) }
        }
        impl<$($gen),+> SweetMut<$ty<$($gen),+>> for &mut $ty<$($gen),+> {
            fn sweet_mut(&mut self) -> &mut $ty<$($gen),+> { $get_mut(self) }
        }
    };
}
