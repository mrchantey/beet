- Response::ok_body should use MimeType not String
- all occurances of Link::new() should be paird with .with_text, not manually adding children!
- MimeLoadTool and MimeRenderTool should also handle binary and json via mime_serde etc

- beet_net add a hyper feature, otherwise server defaults to mini_http_server
- move the beet_stack http_server to beet_net, renamed as `mini_http_server.rs`, and in beet_net make the HttpServer use that one if no lambda or hyper feature flags.
- now the beet_stack http_server will just use the beet_net http_server