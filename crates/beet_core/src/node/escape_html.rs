pub struct EscapeHtml;

impl EscapeHtml {
	/// Escape a string for use in HTML.
	pub fn escape(s: &str) -> String {
		let mut escaped = String::with_capacity(s.len());
		for character in s.chars() {
			match character {
				'&' => escaped.push_str("&amp;"),
				'<' => escaped.push_str("&lt;"),
				'>' => escaped.push_str("&gt;"),
				'"' => escaped.push_str("&quot;"),
				'\'' => escaped.push_str("&#39;"),
				_ => escaped.push(character),
			}
		}
		escaped
	}
	/// Unescape a string from HTML.
	pub fn unescape(s: &str) -> String {
		s.replace("&amp;", "&")
			.replace("&lt;", "<")
			.replace("&gt;", ">")
			.replace("&quot;", "\"")
			.replace("&#39;", "'")
	}
}
