use anyhow::{Context, Result, anyhow};
use async_tungstenite::tokio::connect_async;
use futures::{FutureExt, SinkExt, StreamExt, future::BoxFuture};
use reqwest::Client;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

/// Represents an outbound BiDi command waiting for a response.
struct Pending {
    resolve: tokio::sync::oneshot::Sender<Value>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Create classic WebDriver session requesting webSocketUrl.
    let http_client = Client::new();
    let session_resp: Value = http_client
        .post("http://localhost:9515/session")
        .json(&json!({
            "capabilities": {
                "alwaysMatch": {
                    "browserName": "chrome",
                    // 'webSocketUrl': true signals we want the BiDi endpoint reported back (ChromeDriver supports this).
                    "webSocketUrl": true,
                    // Optional: run headless
                    "goog:chromeOptions": {
                        "args": [
                        // "--headless=new",
                        "--disable-gpu"]
                    }
                }
            }
        }))
        .send()
        .await
        .context("creating WebDriver session")?
        .error_for_status()?
        .json()
        .await
        .context("parsing session JSON")?;

    let capabilities = session_resp
        .get("value")
        .and_then(|v| v.get("capabilities"))
        .ok_or_else(|| anyhow!("missing capabilities in session response: {session_resp}"))?;

    let websocket_url = capabilities
        .get("webSocketUrl")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Driver did not return webSocketUrl (BiDi not supported/enabled)"))?
        .to_string();

    let session_id = session_resp
        .get("value")
        .and_then(|v| v.get("sessionId"))
        .and_then(|v| v.as_str())
        .unwrap_or("<unknown>");

    println!("Got session id: {session_id}");
    println!("BiDi websocket: {websocket_url}");

    // 2. Connect to BiDi websocket.
    let (ws_stream, _resp) = connect_async(&websocket_url)
        .await
        .with_context(|| format!("connecting websocket {websocket_url}"))?;
    println!("Connected to BiDi.");

    let (ws_sink, mut ws_stream) = ws_stream.split();

    // 3. Infrastructure: channel to send commands and await responses.
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<(u64, String)>();
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<Value>();

    let pending_map: Arc<tokio::sync::Mutex<HashMap<u64, Pending>>> =
        Arc::new(tokio::sync::Mutex::new(HashMap::new()));

    // Task: writer - forwards outbound text frames.
    // Move the sink into the writer task (SplitSink doesn't implement Clone).
    tokio::spawn(async move {
        let mut ws_sink = ws_sink;
        while let Some((id, raw)) = cmd_rx.recv().await {
            if let Err(err) = ws_sink
                .send(async_tungstenite::tungstenite::Message::Text(raw))
                .await
            {
                eprintln!("Send error for id {id}: {err}");
                break;
            }
        }
    });

    // Task: reader - dispatches responses vs events.
    let pending_map_reader = pending_map.clone();
    let event_tx_reader = event_tx.clone();
    tokio::spawn(async move {
        while let Some(msg_result) = ws_stream.next().await {
            let msg = match msg_result {
                Ok(m) => m,
                Err(err) => {
                    eprintln!("WebSocket read error: {err}");
                    break;
                }
            };
            if !msg.is_text() {
                continue;
            }
            let text = msg.into_text().unwrap();
            let parsed: Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(err) => {
                    eprintln!("JSON parse error: {err} payload={text}");
                    continue;
                }
            };

            // BiDi events have "method" but no "id" response field.
            if parsed.get("id").is_some() {
                // Response to a command
                if let Some(id) = parsed.get("id").and_then(|v| v.as_u64()) {
                    let pending_opt = {
                        let mut guard = pending_map_reader.lock().await;
                        guard.remove(&id)
                    };
                    if let Some(pending) = pending_opt {
                        let _ = pending.resolve.send(parsed);
                    }
                }
            } else if parsed.get("method").is_some() {
                // Event
                let _ = event_tx_reader.send(parsed);
            } else {
                eprintln!("Unknown message form: {parsed}");
            }
        }
    });

    // Command helper
    let send_command = |method: &str, params: Value| -> BoxFuture<'static, Result<Value>> {
        let method = method.to_string();
        let params = params;
        let cmd_tx = cmd_tx.clone();
        let pending_map = pending_map.clone();
        async move {
            static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
            let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let (tx, rx) = tokio::sync::oneshot::channel();
            {
                let mut guard = pending_map.lock().await;
                guard.insert(id, Pending { resolve: tx });
            }
            let payload = json!({
                "id": id,
                "method": method,
                "params": params
            });
            let raw = serde_json::to_string(&payload)?;
            cmd_tx
                .send((id, raw))
                .map_err(|_| anyhow!("command channel closed"))?;
            let resp = rx.await.map_err(|_| anyhow!("response dropped"))?;
            // BiDi success shape: {"id":..,"result":{...}}
            if let Some(err_obj) = resp.get("error") {
                return Err(anyhow!("BiDi error for {method}: {err_obj}"));
            }
            Ok(resp)
        }
        .boxed()
    };

    // 4. Get browsing context list (top-level pages).
    let tree_resp = send_command("browsingContext.getTree", json!({"maxDepth": 0})).await?;
    let contexts = tree_resp
        .get("result")
        .and_then(|r| r.get("contexts"))
        .and_then(|c| c.as_array())
        .ok_or_else(|| anyhow!("No contexts array in getTree response: {tree_resp}"))?;
    if contexts.is_empty() {
        return Err(anyhow!("No browsing contexts returned"));
    }
    let context_id = contexts[0]
        .get("context")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing context id field"))?;
    println!("Using context: {context_id}");

    // 5. (Optional) subscribe to load events for this context.
    let _sub_resp = send_command(
        "session.subscribe",
        json!({
            "events": ["browsingContext.load"],
            "contexts": [context_id]
        }),
    )
    .await?;

    // 6. Navigate
    let _nav_resp = send_command(
        "browsingContext.navigate",
        json!({
            "context": context_id,
            "url": "https://example.com",
            "wait": "complete"  // ask to wait for load event internally (driver-dependent)
        }),
    )
    .await?;

    // 7. Wait for a load event (or timeout fallback).
    let mut loaded = false;
    let timeout = tokio::time::timeout(Duration::from_secs(10), async {
        while let Some(ev) = event_rx.recv().await {
            if ev.get("method").and_then(|m| m.as_str()) == Some("browsingContext.load") {
                let ev_ctx = ev
                    .get("params")
                    .and_then(|p| p.get("context"))
                    .and_then(|v| v.as_str());
                if ev_ctx == Some(context_id) {
                    println!("Load event received.");
                    loaded = true;
                    break;
                }
            }
        }
    });
    let _ = timeout.await;
    if !loaded {
        println!("Did not observe browsingContext.load event; proceeding anyway.");
    }

    // 8. Evaluate heading text.
    let eval_resp = send_command(
        "script.evaluate",
        json!({
            "expression": "document.querySelector('h1')?.textContent",
            "target": { "context": context_id },
            "awaitPromise": true,
            "resultOwnership": "root"
        }),
    )
    .await?;

    let heading_val = eval_resp
        .get("result")
        .and_then(|r| r.get("result"))
        .and_then(|rv| rv.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("<missing>");

    println!("Extracted heading: {heading_val}");

    if heading_val != "Example Domain" {
        return Err(anyhow!(
            "Heading mismatch. Expected 'Example Domain' got '{heading_val}'"
        ));
    }
    println!("Heading assertion passed.");

    // 9. Click the anchor using script (simplest).
    let click_resp = send_command(
        "script.evaluate",
        json!({
            "expression": "document.querySelector('a')?.click(); 'clicked';",
            "target": { "context": context_id },
            "awaitPromise": true
        }),
    )
    .await?;
    if click_resp.get("result").is_none() {
        return Err(anyhow!("Anchor click script had no result"));
    }
    println!("Anchor click dispatched.");

    // 10. (Optional) verify navigation change (the example.com link usually points to IANA).
    // Give a small delay for navigation.
    sleep(Duration::from_secs(1)).await;
    let url_resp = send_command(
        "script.evaluate",
        json!({
            "expression": "location.href",
            "target": { "context": context_id },
            "awaitPromise": true
        }),
    )
    .await?;
    let new_url = url_resp
        .get("result")
        .and_then(|r| r.get("result"))
        .and_then(|rv| rv.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("<unknown>");
    println!("New URL after click: {new_url}");

    // 11. Clean up: delete session via classic WebDriver (so ChromeDriver quits browser).
    // (Optional, but polite.)
    let delete_url = format!("http://localhost:9515/session/{session_id}");
    let _ = http_client.delete(&delete_url).send().await;

    println!("Done.");
    Ok(())
}
