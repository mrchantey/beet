//! From `pretty_assertions` crate, with some modifications.
//! https://github.com/rust-pretty-assertions/rust-pretty-assertions/blob/main/pretty_assertions/src/printer.rs
use crate::matchers::paint_ext;
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
	let diff = ::diff::chars(left, right);
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
			::diff::Result::Both(value, _) => {
				writer.write_with_style(value, light)?
			}
			::diff::Result::Left(value) => {
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
			::diff::Result::Both(value, _) => {
				writer.write_with_style(value, light)?
			}
			::diff::Result::Right(value) => {
				writer.write_with_style(value, heavy)?
			}
			_ => (),
		}
	}
	writer.finish()
}

#[cfg(test)]
mod test {
	use crate::matchers::paint_ext;

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
