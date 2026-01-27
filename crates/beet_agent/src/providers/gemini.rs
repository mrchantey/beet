//! Gemini provider supporting the OpenResponses API.
//!
//! Gemini provides cloud-based LLM inference. This provider translates
//! OpenResponses requests/responses to and from Gemini's native format.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::tasks::BoxedFuture;
use futures::Stream;
use serde_json::Value;
use serde_json::json;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

impl GeminiProvider {
	/// Gemini 2.5 Flash - fast and efficient.
	pub const GEMINI_2_5_FLASH: &str = "gemini-2.5-flash";
	/// Gemini 2.5 Flash with image generation support.
	pub const GEMINI_2_5_FLASH_IMAGE: &str = "gemini-2.5-flash-preview-05-20";
	/// Gemini 2.5 Pro - most capable.
	pub const GEMINI_2_5_PRO: &str = "gemini-2.5-pro";
}

const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// An OpenResponses-compatible provider for Google Gemini API.
///
/// Gemini API key must be set via the `GEMINI_API_KEY` environment variable.
pub struct GeminiProvider {
	api_key: String,
}

impl Default for GeminiProvider {
	fn default() -> Self { Self::new().unwrap() }
}

impl GeminiProvider {
	/// Creates a new provider with the API key from the environment.
	pub fn new() -> Result<Self> {
		let api_key = env_ext::var("GEMINI_API_KEY")?;
		Ok(Self { api_key })
	}

	/// Converts an OpenResponses request to Gemini format.
	fn convert_request(
		&self,
		request: &openresponses::RequestBody,
	) -> Result<(String, Value)> {
		let (contents, system_messages) = self.convert_input(&request.input)?;

		let mut body = json!({
			"contents": contents,
		});

		// Collect system instruction parts
		let mut system_parts = Vec::new();

		// Add system messages from input items
		for msg in system_messages {
			system_parts.push(json!({ "text": msg }));
		}

		// Add system instruction if provided
		if let Some(instructions) = &request.instructions {
			system_parts.push(json!({ "text": instructions }));
		}

		// Set systemInstruction if we have any system content
		if !system_parts.is_empty() {
			body["systemInstruction"] = json!({
				"parts": system_parts
			});
		}

		// Add generation config
		let mut generation_config = json!({});
		if let Some(temp) = request.temperature {
			generation_config["temperature"] = json!(temp);
		}
		if let Some(max_tokens) = request.max_output_tokens {
			generation_config["maxOutputTokens"] = json!(max_tokens);
		}
		if let Some(top_p) = request.top_p {
			generation_config["topP"] = json!(top_p);
		}
		if generation_config != json!({}) {
			body["generationConfig"] = generation_config;
		}

		// Add tools if present
		if let Some(tools) = &request.tools {
			let function_declarations: Vec<Value> = tools
				.iter()
				.map(|tool| {
					let mut func = json!({
						"name": tool.name,
					});
					if let Some(desc) = &tool.description {
						func["description"] = json!(desc);
					}
					if let Some(params) = &tool.parameters {
						func["parameters"] = params.clone();
					}
					func
				})
				.collect();
			body["tools"] =
				json!([{ "functionDeclarations": function_declarations }]);
		}

		Ok((request.model.clone(), body))
	}

	/// Converts OpenResponses input to Gemini contents format.
	fn convert_input(
		&self,
		input: &openresponses::request::Input,
	) -> Result<(Vec<Value>, Vec<String>)> {
		match input {
			openresponses::request::Input::Text(text) => Ok((
				vec![json!({
					"role": "user",
					"parts": [{ "text": text }]
				})],
				Vec::new(),
			)),
			openresponses::request::Input::Items(items) => {
				let mut contents = Vec::new();
				let mut system_messages = Vec::new();
				for item in items {
					match item {
						openresponses::request::InputItem::Message(msg) => {
							let role = match msg.role {
								openresponses::MessageRole::User => "user",
								openresponses::MessageRole::Assistant => "model",
								openresponses::MessageRole::System
								| openresponses::MessageRole::Developer => {
									// Extract system/developer messages for systemInstruction
									if let openresponses::request::MessageContent::Text(text) = &msg.content {
										system_messages.push(text.clone());
									} else if let openresponses::request::MessageContent::Parts(parts) = &msg.content {
										// Collect text from parts
										for part in parts {
											if let Some(text) = part.as_text() {
												system_messages.push(text.to_string());
											}
										}
									}
									continue;
								}
							};

							let parts = self.convert_message_content(&msg.content)?;
							if !parts.is_empty() {
								contents.push(json!({
									"role": role,
									"parts": parts
								}));
							}
						}
						openresponses::request::InputItem::FunctionCall(fc) => {
							contents.push(json!({
								"role": "model",
								"parts": [{
									"functionCall": {
										"name": fc.name,
										"args": serde_json::from_str::<Value>(&fc.arguments)
											.unwrap_or(json!({}))
									}
								}]
							}));
						}
						openresponses::request::InputItem::FunctionCallOutput(fco) => {
							contents.push(json!({
								"role": "user",
								"parts": [{
									"functionResponse": {
										"name": fco.call_id,
										"response": {
											"result": fco.output
										}
									}
								}]
							}));
						}
						openresponses::request::InputItem::ItemReference(_) => {
							// Skip item references - not supported in Gemini
						}
						openresponses::request::InputItem::Reasoning(_) => {
							// Skip reasoning items - not directly supported
						}
					}
				}
				Ok((contents, system_messages))
			}
		}
	}

	/// Converts message content to Gemini parts.
	fn convert_message_content(
		&self,
		content: &openresponses::request::MessageContent,
	) -> Result<Vec<Value>> {
		match content {
			openresponses::request::MessageContent::Text(text) => {
				Ok(vec![json!({ "text": text })])
			}
			openresponses::request::MessageContent::Parts(parts) => {
				let mut gemini_parts = Vec::new();
				for part in parts {
					match part {
						openresponses::ContentPart::InputText(text) => {
							gemini_parts.push(json!({ "text": text.text }));
						}
						openresponses::ContentPart::OutputText(text) => {
							gemini_parts.push(json!({ "text": text.text }));
						}
						openresponses::ContentPart::InputImage(img) => {
							let url = &img.image_url;
							if url.starts_with("data:") {
								// Parse data URL
								let parts: Vec<&str> =
									url.splitn(2, ',').collect();
								if parts.len() == 2 {
									let mime_info = parts[0]
										.strip_prefix("data:")
										.unwrap_or("image/png;base64")
										.split(';')
										.next()
										.unwrap_or("image/png");
									gemini_parts.push(json!({
										"inlineData": {
											"mimeType": mime_info,
											"data": parts[1]
										}
									}));
								}
							} else {
								// External URL
								gemini_parts.push(json!({
									"fileData": {
										"mimeType": "image/jpeg",
										"fileUri": url
									}
								}));
							}
						}
						openresponses::ContentPart::InputFile(file) => {
							if let Some(url) = &file.file_url {
								gemini_parts.push(json!({
									"fileData": {
										"fileUri": url
									}
								}));
							} else if let Some(data) = &file.file_data {
								// Base64 encoded file
								let mime_type = file
									.filename
									.as_ref()
									.map(|f| {
										mime_guess::from_path(f)
											.first_or_octet_stream()
											.essence_str()
											.to_string()
									})
									.unwrap_or_else(|| {
										"application/octet-stream".to_string()
									});
								gemini_parts.push(json!({
									"inlineData": {
										"mimeType": mime_type,
										"data": data
									}
								}));
							}
						}
						openresponses::ContentPart::Refusal(_) => {
							// Skip refusals
						}
						_ => {
							// Skip other content types
						}
					}
				}
				Ok(gemini_parts)
			}
		}
	}

	/// Builds the request for non-streaming.
	fn build_request(&self, model: &str, body: &Value) -> Result<Request> {
		let url = format!("{BASE_URL}/models/{model}:generateContent");
		Request::post(url)
			.with_header("x-goog-api-key", &self.api_key)
			.with_json_body(body)?
			.xok()
	}

	/// Builds the request for streaming.
	fn build_stream_request(
		&self,
		model: &str,
		body: &Value,
	) -> Result<Request> {
		let url =
			format!("{BASE_URL}/models/{model}:streamGenerateContent?alt=sse");
		Request::post(url)
			.with_header("x-goog-api-key", &self.api_key)
			.with_json_body(body)?
			.xok()
	}

	/// Converts a Gemini response to OpenResponses format.
	fn convert_response(
		&self,
		model: &str,
		gemini_response: Value,
	) -> Result<openresponses::ResponseBody> {
		let candidates = gemini_response["candidates"]
			.as_array()
			.ok_or_else(|| bevyhow!("No candidates in Gemini response"))?;

		if candidates.is_empty() {
			bevybail!("Empty candidates array in Gemini response");
		}

		let candidate = &candidates[0];
		let parts = candidate["content"]["parts"]
			.as_array()
			.map(|a| a.to_vec())
			.unwrap_or_default();

		let mut output = Vec::new();
		let mut text_content = String::new();

		for (idx, part) in parts.iter().enumerate() {
			if let Some(text) = part["text"].as_str() {
				text_content.push_str(text);
			} else if let Some(fc) = part["functionCall"].as_object() {
				let name = fc["name"].as_str().unwrap_or("unknown").to_string();
				let args = fc
					.get("args")
					.map(|a| serde_json::to_string(a).unwrap_or_default())
					.unwrap_or_else(|| "{}".to_string());

				output.push(openresponses::OutputItem::FunctionCall(
					openresponses::FunctionCall {
						id: format!("fc_{idx}"),
						call_id: format!("call_{idx}"),
						name,
						arguments: args,
						status: Some(
							openresponses::FunctionCallStatus::Completed,
						),
					},
				));
			}
		}

		// Add text message if we have any
		if !text_content.is_empty() {
			output.push(openresponses::OutputItem::Message(
				openresponses::Message {
					id: "msg_0".to_string(),
					role: openresponses::MessageRole::Assistant,
					content: vec![openresponses::OutputContent::OutputText(
						openresponses::OutputText::new(text_content),
					)],
					status: openresponses::MessageStatus::Completed,
				},
			));
		}

		// Extract usage info
		let usage = gemini_response["usageMetadata"].as_object().map(|u| {
			openresponses::Usage::new(
				u["promptTokenCount"].as_u64().unwrap_or(0) as u32,
				u["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
			)
		});

		Ok(Self::create_response_body(model, output, usage))
	}

	/// Helper to create a ResponseBody with default fields.
	fn create_response_body(
		model: &str,
		output: Vec<openresponses::OutputItem>,
		usage: Option<openresponses::Usage>,
	) -> openresponses::ResponseBody {
		openresponses::ResponseBody {
			id: format!("gemini_{}", time_ext::now_millis()),
			object: "response".to_string(),
			created_at: None,
			completed_at: None,
			status: openresponses::response::Status::Completed,
			incomplete_details: None,
			model: Some(model.to_string()),
			previous_response_id: None,
			instructions: None,
			output,
			error: None,
			tools: Vec::new(),
			tool_choice: None,
			truncation: None,
			parallel_tool_calls: None,
			text: None,
			top_p: None,
			presence_penalty: None,
			frequency_penalty: None,
			top_logprobs: None,
			temperature: None,
			reasoning: None,
			usage,
			max_output_tokens: None,
			max_tool_calls: None,
			store: None,
			background: None,
			service_tier: None,
			metadata: None,
			safety_identifier: None,
			prompt_cache_key: None,
		}
	}

	/// Helper to create a Message for streaming.
	fn create_message(
		id: &str,
		text: String,
		status: openresponses::MessageStatus,
	) -> openresponses::Message {
		openresponses::Message {
			id: id.to_string(),
			role: openresponses::MessageRole::Assistant,
			content: vec![openresponses::OutputContent::OutputText(
				openresponses::OutputText::new(text),
			)],
			status,
		}
	}
}

impl ModelProvider for GeminiProvider {
	fn provider_slug(&self) -> &'static str { "gemini" }

	fn default_small_model(&self) -> &'static str { Self::GEMINI_2_5_FLASH }
	fn default_tool_model(&self) -> &'static str { Self::GEMINI_2_5_FLASH }
	fn default_large_model(&self) -> &'static str { Self::GEMINI_2_5_PRO }

	fn send(
		&self,
		request: openresponses::RequestBody,
	) -> BoxedFuture<'_, Result<openresponses::ResponseBody>> {
		Box::pin(async move {
			let (model, body) = self.convert_request(&request)?;
			let response = self
				.build_request(&model, &body)?
				.send()
				.await?
				.into_result()
				.await?
				.json::<Value>()
				.await?;

			self.convert_response(&model, response)
		})
	}

	fn stream(
		&self,
		request: openresponses::RequestBody,
	) -> BoxedFuture<'_, Result<StreamingEventStream>> {
		Box::pin(async move {
			let (model, body) = self.convert_request(&request)?;
			let raw_stream = self
				.build_stream_request(&model, &body)?
				.send()
				.await?
				.event_source_raw()
				.await?;

			let stream: StreamingEventStream =
				Box::pin(GeminiStream::new(raw_stream, model));
			stream.xok()
		})
	}
}

/// A stream that converts Gemini SSE events to OpenResponses StreamingEvent values.
struct GeminiStream<S> {
	inner: S,
	model: String,
	done: bool,
	response_created: bool,
	item_added: bool,
	sequence: i64,
	accumulated_text: String,
	event_buffer: Vec<openresponses::StreamingEvent>,
}

impl<S> GeminiStream<S> {
	fn new(inner: S, model: String) -> Self {
		Self {
			inner,
			model,
			done: false,
			response_created: false,
			item_added: false,
			sequence: 0,
			accumulated_text: String::new(),
			event_buffer: Vec::new(),
		}
	}

	fn next_sequence(&mut self) -> i64 {
		let seq = self.sequence;
		self.sequence += 1;
		seq
	}
}

impl<S, E> Stream for GeminiStream<S>
where
	S: Stream<
			Item = std::result::Result<
				beet_net::exports::eventsource_stream::Event,
				E,
			>,
		> + Unpin
		+ Send,
	E: std::fmt::Display,
{
	type Item = Result<openresponses::StreamingEvent>;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Self::Item>> {
		// Return buffered events first
		if let Some(event) = self.event_buffer.pop() {
			return Poll::Ready(Some(Ok(event)));
		}

		if self.done {
			return Poll::Ready(None);
		}

		match Pin::new(&mut self.inner).poll_next(cx) {
			Poll::Ready(Some(Ok(event))) => {
				// Parse Gemini event
				let body: Value = match serde_json::from_str(&event.data) {
					Ok(v) => v,
					Err(err) => {
						return Poll::Ready(Some(Err(bevyhow!(
							"Failed to parse Gemini event: {}",
							err
						))));
					}
				};

				let mut events = Vec::new();

				// Send response.created on first event
				if !self.response_created {
					self.response_created = true;
					let seq = self.next_sequence();
					let response = GeminiProvider::create_response_body(
						&self.model,
						vec![],
						None,
					);
					events.push(
						openresponses::StreamingEvent::ResponseCreated(
							openresponses::streaming::ResponseCreatedEvent {
								sequence_number: seq,
								response,
							},
						),
					);
				}

				// Send output_item.added on first content
				if !self.item_added {
					self.item_added = true;
					let seq = self.next_sequence();
					events
						.push(openresponses::StreamingEvent::OutputItemAdded(
						openresponses::streaming::OutputItemAddedEvent {
							sequence_number: seq,
							output_index: 0,
							item: Some(openresponses::OutputItem::Message(
								GeminiProvider::create_message(
									"msg_0",
									String::new(),
									openresponses::MessageStatus::InProgress,
								),
							)),
						},
					));
				}

				// Extract text from Gemini response
				let candidates = body["candidates"].as_array();
				if let Some(candidates) = candidates {
					if let Some(candidate) = candidates.first() {
						if let Some(parts) =
							candidate["content"]["parts"].as_array()
						{
							for part in parts {
								if let Some(text) = part["text"].as_str() {
									// Calculate delta (new text since last)
									let delta =
										if self.accumulated_text.is_empty() {
											text.to_string()
										} else if text.len()
											> self.accumulated_text.len()
										{
											text[self.accumulated_text.len()..]
												.to_string()
										} else {
											text.to_string()
										};
									self.accumulated_text = text.to_string();

									if !delta.is_empty() {
										let seq = self.next_sequence();
										events.push(
											openresponses::StreamingEvent::OutputTextDelta(
												openresponses::streaming::OutputTextDeltaEvent {
													sequence_number: seq,
													item_id: "msg_0".to_string(),
													output_index: 0,
													content_index: 0,
													delta,
													logprobs: Vec::new(),
													obfuscation: None,
												},
											),
										);
									}
								}
							}
						}
					}
				}

				// Check for finish reason
				if let Some(candidates) = candidates {
					if let Some(candidate) = candidates.first() {
						if candidate["finishReason"].as_str().is_some() {
							self.done = true;

							// Send output_item.done
							let seq = self.next_sequence();
							events.push(openresponses::StreamingEvent::OutputItemDone(
								openresponses::streaming::OutputItemDoneEvent {
									sequence_number: seq,
									output_index: 0,
									item: Some(openresponses::OutputItem::Message(
										GeminiProvider::create_message("msg_0", self.accumulated_text.clone(), openresponses::MessageStatus::Completed),
									)),
								},
							));

							// Extract usage info
							let usage =
								body["usageMetadata"].as_object().map(|u| {
									openresponses::Usage::new(
										u["promptTokenCount"]
											.as_u64()
											.unwrap_or(0) as u32,
										u["candidatesTokenCount"]
											.as_u64()
											.unwrap_or(0) as u32,
									)
								});

							// Send response.completed
							let seq = self.next_sequence();
							let output =
								vec![openresponses::OutputItem::Message(
									GeminiProvider::create_message(
										"msg_0",
										self.accumulated_text.clone(),
										openresponses::MessageStatus::Completed,
									),
								)];
							let mut response =
								GeminiProvider::create_response_body(
									&self.model,
									output,
									usage,
								);
							response.status =
								openresponses::response::Status::Completed;
							events.push(
								openresponses::StreamingEvent::ResponseCompleted(
									openresponses::streaming::ResponseCompletedEvent {
										sequence_number: seq,
										response,
									},
								),
							);
						}
					}
				}

				// Buffer events in reverse order so we can pop them in correct order
				if !events.is_empty() {
					let mut events_iter = events.into_iter();
					let first_event = events_iter.next().unwrap();
					// Add remaining events to buffer in reverse
					self.event_buffer.extend(events_iter.rev());
					Poll::Ready(Some(Ok(first_event)))
				} else {
					// No events generated, poll again
					cx.waker().wake_by_ref();
					Poll::Pending
				}
			}
			Poll::Ready(Some(Err(err))) => {
				Poll::Ready(Some(Err(bevyhow!("Gemini SSE error: {}", err))))
			}
			Poll::Ready(None) => {
				if !self.done && self.response_created {
					// Stream ended without finish reason, send completion
					self.done = true;
					let seq = self.next_sequence();
					let output = vec![openresponses::OutputItem::Message(
						GeminiProvider::create_message(
							"msg_0",
							self.accumulated_text.clone(),
							openresponses::MessageStatus::Completed,
						),
					)];
					let mut response = GeminiProvider::create_response_body(
						&self.model,
						output,
						None,
					);
					response.status =
						openresponses::response::Status::Completed;
					Poll::Ready(Some(Ok(
						openresponses::StreamingEvent::ResponseCompleted(
							openresponses::streaming::ResponseCompletedEvent {
								sequence_number: seq,
								response,
							},
						),
					)))
				} else {
					Poll::Ready(None)
				}
			}
			Poll::Pending => Poll::Pending,
		}
	}
}
