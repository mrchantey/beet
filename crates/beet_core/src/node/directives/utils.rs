pub struct NodeUtils;


const ABS_URL_PREFIXES: [&str; 15] = [
	"/",
	"http://",
	"https://",
	"file://",
	"data:",
	"mailto:",
	"tel:",
	"javascript:",
	"ftp://",
	"ws://",
	"wss://",
	"blob:",
	"cid:",
	"about:",
	"chrome:",
];



impl NodeUtils {
	pub fn is_relative_url(url: &str) -> bool {
		!ABS_URL_PREFIXES
			.iter()
			.any(|prefix| url.starts_with(prefix))
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		NodeUtils::is_relative_url("style.css")
			.xpect()
			.to_be_true();
		NodeUtils::is_relative_url("../style.css")
			.xpect()
			.to_be_true();
		NodeUtils::is_relative_url("/style.css")
			.xpect()
			.to_be_false();
		NodeUtils::is_relative_url("https://example.com")
			.xpect()
			.to_be_false();
	}
}
