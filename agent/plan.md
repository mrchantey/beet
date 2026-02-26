Keep iterating on headers, ie our new `header_map.rs`

- remove the request/response/requestparts/responseparts header helper methods. now the only way to interact with headers is by interacting with the headers object directly:
request.headers.get()
response.headers.get()


refactor ExchangeFormat to be just a utility module

```
src/exchange/mime_serde.rs


pub fn serialize(type: MimeType, value:T)->Vec<u8>{
	..
}
pub fn deserialize(type: MimeType, bytes: &[u8]){
	..
}

// usage

let a = mime_serde::serialize(..).unwrap()

```