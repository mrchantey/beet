#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::style::*;
use crate::token;

// Ref tokens - map to values

// Typeface family ref tokens
token!(Typeface, TYPEFACE_BRAND, "typeface-brand");
token!(Typeface, TYPEFACE_PLAIN, "typeface-plain");
// Font weight ref tokens
token!(FontWeight, WEIGHT_REGULAR, "weight-regular");
token!(FontWeight, WEIGHT_MEDIUM, "weight-medium");
token!(FontWeight, WEIGHT_BOLD, "weight-bold");


// Sys tokens - map to ref tokens


token!(Typography, DISPLAY_LARGE,   "display-large");
token!(Typography, DISPLAY_MEDIUM,  "display-medium");
token!(Typography, DISPLAY_SMALL,   "display-small");
token!(Typography, HEADLINE_LARGE,  "headline-large");
token!(Typography, HEADLINE_MEDIUM, "headline-medium");
token!(Typography, HEADLINE_SMALL,  "headline-small");
token!(Typography, TITLE_LARGE,     "title-large");
token!(Typography, TITLE_MEDIUM,    "title-medium");
token!(Typography, TITLE_SMALL,     "title-small");
token!(Typography, BODY_LARGE,      "body-large");
token!(Typography, BODY_MEDIUM,     "body-medium");
token!(Typography, BODY_SMALL,      "body-small");
token!(Typography, LABEL_LARGE,     "label-large");
token!(Typography, LABEL_MEDIUM,    "label-medium");
token!(Typography, LABEL_SMALL,     "label-small");



/// Returns a [`TokenStore`] populated with the 15 Material Design 3
/// typescale definitions, using ref token values for typeface and weight.
pub fn default_typography() -> TokenStore {
	TokenStore::new()
    .with(TYPEFACE_PLAIN, Typeface::new([
    	"Roboto",
      "system-ui",
      "-apple-system",
      "BlinkMacSystemFont",
      "Segoe UI",
      "sans-serif",
    ]))
    .with(TYPEFACE_BRAND, Typeface::new([
    	"Google Sans",
      "Product Sans",
      "Inter",
      "Work Sans",
      "system-ui",
      "sans-serif",
    ]))
    .with(WEIGHT_REGULAR, 	FontWeight::Absolute(400))
    .with(WEIGHT_MEDIUM, 	FontWeight::Absolute(500))
    .with(WEIGHT_BOLD, 		FontWeight::Absolute(700))
    .with(DISPLAY_LARGE,		Typography { typeface: TYPEFACE_BRAND, size: Length::rem(3.5625), weight: WEIGHT_REGULAR, line_height:None,letter_spacing:None})
    .with(DISPLAY_MEDIUM, 	Typography { typeface: TYPEFACE_BRAND, size: Length::rem(2.8125), weight: WEIGHT_REGULAR, line_height:None,letter_spacing:None})
    .with(DISPLAY_SMALL,  	Typography { typeface: TYPEFACE_BRAND, size: Length::rem(2.25), 	weight: WEIGHT_REGULAR, line_height:None,letter_spacing:None})
    .with(HEADLINE_LARGE, 	Typography { typeface: TYPEFACE_PLAIN, size: Length::rem(2.0), 	weight: WEIGHT_REGULAR, line_height:None,letter_spacing:None})
    .with(HEADLINE_MEDIUM, 	Typography { typeface: TYPEFACE_PLAIN, size: Length::rem(1.75), 	weight: WEIGHT_REGULAR, line_height:None,letter_spacing:None})
    .with(HEADLINE_SMALL, 	Typography { typeface: TYPEFACE_PLAIN, size: Length::rem(1.5), 	weight: WEIGHT_REGULAR, line_height:None,letter_spacing:None})
    .with(TITLE_LARGE, 			Typography { typeface: TYPEFACE_PLAIN, size: Length::rem(1.375), weight: WEIGHT_REGULAR, line_height:None,letter_spacing:None})
    .with(TITLE_MEDIUM,			Typography { typeface: TYPEFACE_PLAIN, size: Length::rem(1.0),   weight: WEIGHT_MEDIUM, line_height:None,letter_spacing:None})
    .with(TITLE_SMALL, 			Typography { typeface: TYPEFACE_PLAIN, size: Length::rem(0.875), weight: WEIGHT_MEDIUM, line_height:None,letter_spacing:None})
    .with(BODY_LARGE,  			Typography { typeface: TYPEFACE_PLAIN, size: Length::rem(1.0),   weight: WEIGHT_REGULAR, line_height:None,letter_spacing:None})
    .with(BODY_MEDIUM, 			Typography { typeface: TYPEFACE_PLAIN, size: Length::rem(0.875), weight: WEIGHT_REGULAR, line_height:None,letter_spacing:None})
    .with(BODY_SMALL,  			Typography { typeface: TYPEFACE_PLAIN, size: Length::rem(0.75),  weight: WEIGHT_REGULAR, line_height:None,letter_spacing:None})
    .with(LABEL_LARGE,  		Typography { typeface: TYPEFACE_PLAIN, size: Length::rem(0.875),  weight: WEIGHT_MEDIUM, line_height:None,letter_spacing:None})
    .with(LABEL_MEDIUM, 		Typography { typeface: TYPEFACE_PLAIN, size: Length::rem(0.75),   weight: WEIGHT_MEDIUM, line_height:None,letter_spacing:None})
    .with(LABEL_SMALL,  		Typography { typeface: TYPEFACE_PLAIN, size: Length::rem(0.6875), weight: WEIGHT_MEDIUM, line_height:None,letter_spacing:None})
}
