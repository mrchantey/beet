#[macro_export]
macro_rules! synhow {
 ($span:expr, $($arg:tt)*) => {
  syn::Error::new_spanned($span, std::format!($($arg)*))
 };
}

#[macro_export]
macro_rules! synbail {
 ($span:expr, $($arg:tt)*) => {
  return Err(syn::Error::new_spanned($span, std::format!($($arg)*)))
 };
}
