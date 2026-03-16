see [@url.rs](file:///home/pete/me/beet/crates/beet_net/src/exchange/url.rs) DataUrl. lets remove DataUrl, instead all this 'into url then decode' behavior should be in a single step in MediaBytes.


```rust
// src/exchangey/media_bytes_ext.rs

#[extend::ext(name=MediaBytesUrlExt)]
impl MediaBytes{

	fn from_url(url:&'a Url)->Self<'a>{
		// 1. get the parts 
		// 2. determine encoding (only url encoded/base64)
		// 3. decode immediately to bytes.	
	}
	
	/// Creates a data url with the media type,
	/// encoded as base64
	fn into_url(&self)->Url{
		
	}
}

// and now remove url from_data etc, we dont support
// percentage url encoding, only decoding.
```

- at the end, No DataUrl, no DataUrlEncoding..
