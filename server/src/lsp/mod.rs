//! A language server, run alongside each language's usual server, that answers
//! hovers over comments with their translation.

mod documents;
mod transport;

use std::{sync::Arc, time::Instant};

use anyhow::Result;
use serde_json::{json, Value};
use tokio::{io::BufReader, sync::mpsc};

use crate::{comment::find_comment, hover::HoverTranslator};
use documents::{offset, position, Documents, Encoding};
use transport::{read_message, write_message};

/// Serves stdin/stdout until `exit`, and returns whether `shutdown` came first.
pub async fn serve(hover: Arc<HoverTranslator>) -> Result<bool> {
    // All writes to Zed go through one task so that frames never interleave.
    let (client_tx, mut client_rx) = mpsc::unbounded_channel::<Value>();
    let client_writer = tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(message) = client_rx.recv().await {
            write_message(&mut stdout, &serde_json::to_vec(&message)?).await?;
        }
        anyhow::Ok(())
    });
    let respond = |id: Value, result: Value| {
        let _ = client_tx.send(json!({ "jsonrpc": "2.0", "id": id, "result": result }));
    };

    let mut documents = Documents::default();
    let mut shut_down = false;
    let mut stdin = BufReader::new(tokio::io::stdin());
    while let Some(body) = read_message(&mut stdin).await? {
        let Ok(message) = serde_json::from_slice::<Value>(&body) else {
            continue;
        };
        let method = message["method"].as_str().unwrap_or_default();
        match (method, message.get("id").cloned()) {
            ("initialize", Some(id)) => {
                documents.encoding = negotiate_encoding(&message["params"]);
                let encoding = match documents.encoding {
                    Encoding::Utf8 => "utf-8",
                    Encoding::Utf16 => "utf-16",
                    Encoding::Utf32 => "utf-32",
                };
                respond(
                    id,
                    json!({
                        "capabilities": {
                            "positionEncoding": encoding,
                            // Full open/close notifications and incremental changes.
                            "textDocumentSync": { "openClose": true, "change": 2 },
                            "hoverProvider": true,
                        },
                        "serverInfo": { "name": env!("CARGO_PKG_NAME"), "version": env!("CARGO_PKG_VERSION") },
                    }),
                );
            }
            ("shutdown", Some(id)) => {
                shut_down = true;
                respond(id, Value::Null);
            }
            ("exit", None) => break,
            ("textDocument/hover", Some(id)) => {
                log!("hover request {id}");
                let comment = hovered_comment(&documents, &message["params"]);
                let message = json!({ "jsonrpc": "2.0", "id": id, "result": null });
                let Some((text, range)) = comment else {
                    let _ = client_tx.send(message);
                    continue;
                };
                let hover = hover.clone();
                let client_tx = client_tx.clone();
                tokio::spawn(async move {
                    let started = Instant::now();
                    let message = hover.comment(message, &text, range).await;
                    let outcome = if message["result"].is_null() {
                        "empty"
                    } else {
                        "translated"
                    };
                    log!("hover response {id}: {outcome} in {:?}", started.elapsed());
                    let _ = client_tx.send(message);
                });
            }
            (method, None) if method.starts_with("textDocument/did") => documents.apply(&message),
            // Other requests are not supported; responses and other notifications are ignored.
            (method, Some(id)) if !method.is_empty() => {
                let _ = client_tx.send(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32601, "message": format!("unsupported method: {method}") },
                }));
            }
            _ => {}
        }
    }

    drop(client_tx);
    let _ = client_writer.await;
    Ok(shut_down)
}

/// The first position encoding the client offers that we support, UTF-16 by default.
fn negotiate_encoding(params: &Value) -> Encoding {
    params
        .pointer("/capabilities/general/positionEncodings")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .find(|name| ["utf-8", "utf-16", "utf-32"].contains(name))
        .map_or(Encoding::Utf16, Encoding::from_lsp)
}

/// The text and LSP range of the comment at the position of hover params.
fn hovered_comment(documents: &Documents, params: &Value) -> Option<(String, Value)> {
    let document = documents.get(params["textDocument"]["uri"].as_str()?)?;
    let text = &document.text;
    let at = offset(text, &params["position"], documents.encoding);
    let comment = find_comment(&document.language_id, text, at)?;
    let range = json!({
        "start": position(text, comment.start, documents.encoding),
        "end": position(text, comment.end, documents.encoding),
    });
    Some((comment.text, range))
}
