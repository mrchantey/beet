//! From `pretty_assertions` crate, with some modifications.
//! https://github.com/rust-pretty-assertions/rust-pretty-assertions/blob/main/pretty_assertions/src/printer.rs
use crate::prelude::paint_ext;
use core::fmt;
use nu_ansi_term::Color;
use nu_ansi_term::Style;

const SIGN_LEFT: &str = "Expected:\n";
const SIGN_RIGHT: &str = "Received:\n";

const GREEN_BACKGROUND: u8 = 22;
const RED_BACKGROUND: u8 = 52;

/// Group character styling for an inline diff, to prevent wrapping each single
/// character in terminal styling codes.
///
/// Styles are applied automatically each time a new style is given in `write_with_style`.
struct InlineWriter<'a, Writer> {
	f: &'a mut Writer,
	style: Option<Style>,
}

impl<'a, Writer> InlineWriter<'a, Writer>
where
	Writer: fmt::Write,
{
	fn new(f: &'a mut Writer) -> Self { InlineWriter { f, style: None } }

	/// Push a new character into the buffer, specifying the style it should be written in.
	fn write_with_style(
		&mut self,
		c: &impl fmt::Display,
		style: Style,
	) -> fmt::Result {
		// If the style is the same as previously, just write character
		if self.style == Some(style) {
			write!(self.f, "{}", c)?;
		} else {
			// Close out previous style
			if self.style.is_some() {
				write!(self.f, "\x1b[0m")?;
			}

			// Store new style and start writing it
			if paint_ext::is_enabled() {
				write!(self.f, "{}", style.prefix())?;
			}
			write!(self.f, "{}", c)?;
			self.style = Some(style);
		}
		Ok(())
	}

	/// Finish any existing style and reset to default state.
	fn finish(&mut self) -> fmt::Result {
		// Close out previous style
		if self.style.is_some() && paint_ext::is_enabled() {
			write!(self.f, "\x1b[0m")?;
		}
		writeln!(self.f)?;
		self.style = None;
		Ok(())
	}
}

pub fn inline_diff(expected: &str, received: &str) -> String {
	let mut output = String::new();
	write_inline_diff(&mut output, expected, received)
		.expect("inline diff failed");
	output
}


/// Format a single line to show an inline diff of the two strings given.
///
/// The given strings should not have a trailing newline.
///
/// The output of this function will be two lines, each with a trailing newline.
fn write_inline_diff<TWrite: fmt::Write>(
	f: &mut TWrite,
	left: &str,
	right: &str,
) -> fmt::Result {
	let diff = diff::chars(left, right);
	let mut writer = InlineWriter::new(f);

	// Print the left string on one line, with differences highlighted
	let light = Style::new().fg(Color::Green);
	let heavy = Style::new()
		.fg(Color::Green)
		.on(Color::Fixed(GREEN_BACKGROUND))
		.bold();
	write!(writer.f, "{SIGN_LEFT}\n")?;
	for change in diff.iter() {
		match change {
			diff::Result::Both(value, _) => {
				writer.write_with_style(value, light)?
			}
			diff::Result::Left(value) => {
				writer.write_with_style(value, heavy)?
			}
			_ => (),
		}
	}
	writer.finish()?;

	// Print the right string on one line, with differences highlighted
	let light = Style::new().fg(Color::Red);
	let heavy = Style::new()
		.fg(Color::Red)
		.on(Color::Fixed(RED_BACKGROUND))
		.bold();
	write!(writer.f, "\n{SIGN_RIGHT}\n")?;
	for change in diff.iter() {
		match change {
			diff::Result::Both(value, _) => {
				writer.write_with_style(value, light)?
			}
			diff::Result::Right(value) => {
				writer.write_with_style(value, heavy)?
			}
			_ => (),
		}
	}
	writer.finish()
}


/// Copied from https://crates.io/crates/diff on 2026/01/06
mod diff {
	#![forbid(unsafe_code)]

	/// A fragment of a computed diff.
	#[derive(Clone, Debug, PartialEq, Eq)]
	pub enum Result<T> {
		/// An element that only exists in the left input.
		Left(T),
		/// Elements that exist in both inputs.
		Both(T, T),
		/// An element that only exists in the right input.
		Right(T),
	}

	/// Computes the diff between two slices.
	#[allow(unused)]
	pub fn slice<'a, T: PartialEq>(
		left: &'a [T],
		right: &'a [T],
	) -> Vec<Result<&'a T>> {
		do_diff(left, right, |t| t)
	}

	/// Computes the diff between the lines of two strings.
	#[allow(unused)]
	pub fn lines<'a>(left: &'a str, right: &'a str) -> Vec<Result<&'a str>> {
		let mut diff = do_diff(
			&left.lines().collect::<Vec<_>>(),
			&right.lines().collect::<Vec<_>>(),
			|str| *str,
		);
		// str::lines() does not yield an empty str at the end if the str ends with
		// '\n'. We handle this special case by inserting one last diff item,
		// depending on whether the left string ends with '\n', or the right one,
		// or both.
		match (
			left.as_bytes().last().cloned(),
			right.as_bytes().last().cloned(),
		) {
			(Some(b'\n'), Some(b'\n')) => diff
				.push(Result::Both(&left[left.len()..], &right[right.len()..])),
			(Some(b'\n'), _) => diff.push(Result::Left(&left[left.len()..])),
			(_, Some(b'\n')) => diff.push(Result::Right(&right[right.len()..])),
			_ => {}
		}
		diff
	}

	/// Computes the diff between the chars of two strings.
	pub fn chars<'a>(left: &'a str, right: &'a str) -> Vec<Result<char>> {
		do_diff(
			&left.chars().collect::<Vec<_>>(),
			&right.chars().collect::<Vec<_>>(),
			|char| *char,
		)
	}

	fn do_diff<'a, T, F, U>(
		left: &'a [T],
		right: &'a [T],
		mapper: F,
	) -> Vec<Result<U>>
	where
		T: PartialEq,
		F: Fn(&'a T) -> U,
	{
		let leading_equals = left
			.iter()
			.zip(right.iter())
			.take_while(|(l, r)| l == r)
			.count();
		let trailing_equals = left[leading_equals..]
			.iter()
			.rev()
			.zip(right[leading_equals..].iter().rev())
			.take_while(|(l, r)| l == r)
			.count();

		let mut diff = Vec::with_capacity(left.len().max(right.len()));

		diff.extend(
			left[..leading_equals]
				.iter()
				.zip(&right[..leading_equals])
				.map(|(l, r)| Result::Both(mapper(l), mapper(r))),
		);

		do_naive_diff(
			&left[leading_equals..left.len() - trailing_equals],
			&right[leading_equals..right.len() - trailing_equals],
			&mapper,
			&mut diff,
		);

		diff.extend(
			left[left.len() - trailing_equals..]
				.iter()
				.zip(&right[right.len() - trailing_equals..])
				.map(|(l, r)| Result::Both(mapper(l), mapper(r))),
		);

		diff
	}

	fn do_naive_diff<'a, T, F, U>(
		left: &'a [T],
		right: &'a [T],
		mapper: F,
		diff: &mut Vec<Result<U>>,
	) where
		T: PartialEq,
		F: Fn(&'a T) -> U,
	{
		let mut table = Vec2::new(0u32, [left.len() + 1, right.len() + 1]);

		for (i, l) in left.iter().enumerate() {
			for (j, r) in right.iter().enumerate() {
				table.set(
					[i + 1, j + 1],
					if l == r {
						table.get([i, j]) + 1
					} else {
						*table.get([i, j + 1]).max(table.get([i + 1, j]))
					},
				);
			}
		}

		let start = diff.len();

		let mut i = table.len[0] - 1;
		let mut j = table.len[1] - 1;
		loop {
			if j > 0 && (i == 0 || table.get([i, j]) == table.get([i, j - 1])) {
				j -= 1;
				diff.push(Result::Right(mapper(&right[j])));
			} else if i > 0
				&& (j == 0 || table.get([i, j]) == table.get([i - 1, j]))
			{
				i -= 1;
				diff.push(Result::Left(mapper(&left[i])));
			} else if i > 0 && j > 0 {
				i -= 1;
				j -= 1;
				diff.push(Result::Both(mapper(&left[i]), mapper(&right[j])));
			} else {
				break;
			}
		}

		diff[start..].reverse();
	}

	struct Vec2<T> {
		len: [usize; 2],
		data: Vec<T>,
	}

	impl<T> Vec2<T> {
		#[inline]
		fn new(value: T, len: [usize; 2]) -> Self
		where
			T: Clone,
		{
			Vec2 {
				len,
				data: vec![value; len[0] * len[1]],
			}
		}

		#[inline]
		fn get(&self, index: [usize; 2]) -> &T {
			debug_assert!(index[0] < self.len[0]);
			debug_assert!(index[1] < self.len[1]);
			&self.data[index[0] * self.len[1] + index[1]]
		}

		#[inline]
		fn set(&mut self, index: [usize; 2], value: T) {
			debug_assert!(index[0] < self.len[0]);
			debug_assert!(index[1] < self.len[1]);
			self.data[index[0] * self.len[1] + index[1]] = value;
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	// ANSI terminal codes used in our outputs.
	//
	// Interpolate these into test strings to make expected values easier to read.
	const RED_LIGHT: &str = "\u{1b}[31m";
	const GREEN_LIGHT: &str = "\u{1b}[32m";
	const RED_HEAVY: &str = "\u{1b}[1;48;5;52;31m";
	const GREEN_HEAVY: &str = "\u{1b}[1;48;5;22;32m";
	const RESET: &str = "\u{1b}[0m";

	/// Given that both of our diff printing functions have the same
	/// type signature, we can reuse the same test code for them.
	fn check_printer<TPrint>(
		printer: TPrint,
		left: &str,
		right: &str,
		expected: &str,
	) where
		TPrint: Fn(&mut String, &str, &str) -> fmt::Result,
	{
		paint_ext::set_enabled(true);
		let mut actual = String::new();
		printer(&mut actual, left, right).expect("printer function failed");
		assert_eq!(actual, expected);
	}

	#[test]
	fn write_inline_diff_empty() {
		let left = "";
		let right = "";
		let expected = format!("{SIGN_LEFT}\n\n\n{SIGN_RIGHT}\n\n");

		check_printer(write_inline_diff, left, right, &expected);
	}

	#[test]
	fn write_inline_diff_added() {
		let left = "";
		let right = "polymerase";
		let expected = format!(
			"{SIGN_LEFT}\n\n\n{SIGN_RIGHT}\n{red_heavy}polymerase{reset}\n",
			red_heavy = RED_HEAVY,
			reset = RESET,
		);

		check_printer(write_inline_diff, left, right, &expected);
	}

	#[test]
	fn write_inline_diff_removed() {
		let left = "polyacrylamide";
		let right = "";
		let expected = format!(
			"{SIGN_LEFT}\n{green_heavy}polyacrylamide{reset}\n\n{SIGN_RIGHT}\n\n",
			green_heavy = GREEN_HEAVY,
			reset = RESET,
		);

		check_printer(write_inline_diff, left, right, &expected);
	}

	#[test]
	fn write_inline_diff_changed() {
		let left = "polymerase";
		let right = "polyacrylamide";
		let expected = format!(
			"{SIGN_LEFT}\n{green_light}poly{reset}{green_heavy}me{reset}{green_light}ra{reset}{green_heavy}s{reset}{green_light}e{reset}\n\n{SIGN_RIGHT}\n{red_light}poly{reset}{red_heavy}ac{reset}{red_light}r{reset}{red_heavy}yl{reset}{red_light}a{reset}{red_heavy}mid{reset}{red_light}e{reset}\n",
			red_light = RED_LIGHT,
			green_light = GREEN_LIGHT,
			red_heavy = RED_HEAVY,
			green_heavy = GREEN_HEAVY,
			reset = RESET,
		);

		check_printer(write_inline_diff, left, right, &expected);
	}
}
