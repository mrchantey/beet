pub mod navigate;


/// Default port for a beet server: `8338`
pub const DEFAULT_SERVER_PORT: u16 = 8337;
/// Default port for the webdriver (chromedriver, geckodriver etc): 8339
pub const DEFAULT_WEBDRIVER_PORT: u16 = 8338;
/// Default port for websocket connections (geckodriver only, chromedriver uses default port): 8340
pub const DEFAULT_WEBDRIVER_SESSION_PORT: u16 = 8339;
