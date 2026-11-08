//! Relaying LSP messages between Zed and the wrapped server, rewriting only
//! `textDocument/hover` responses. Open documents are tracked so that hovers
//! over comments can be answered too.

mod documents;
pub mod server;
mod transport;

use std::{
    collections::HashMap,
    process::ExitStatus,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::{io::BufReader, process::Child, sync::mpsc};

use crate::hover::{find_comment, HoverTranslator};
use documents::{offset, position, Documents, Encoding};
use transport::{read_message, write_message};

/// The comment under the cursor of a hover request.
struct HoveredComment {
    text: String,
    range: Value,
}

/// In-flight hover requests, keyed by the JSON representation of their ids.
type HoverRequests = Arc<Mutex<HashMap<String, Option<HoveredComment>>>>;

/// Relays stdin/stdout to the server until it exits, and returns its status.
pub async fn run(mut child: Child, hover: Arc<HoverTranslator>) -> Result<ExitStatus> {
    let server_stdin = child.stdin.take().context("no server stdin")?;
    let mut server_stdout = BufReader::new(child.stdout.take().context("no server stdout")?);
    let hover_requests = HoverRequests::default();
    let documents = Arc::new(Mutex::new(Documents::default()));

    // All writes to Zed go through one task so that frames never interleave.
    let (client_tx, mut client_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let client_writer = tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(body) = client_rx.recv().await {
            write_message(&mut stdout, &body).await?;
        }
        anyhow::Ok(())
    });

    tokio::spawn(forward_client_messages(
        server_stdin,
        hover_requests.clone(),
        documents.clone(),
    ));

    while let Some(body) = read_message(&mut server_stdout).await? {
        let response = serde_json::from_slice::<Value>(&body)
            .ok()
            .filter(|message| message.get("method").is_none());
        if let Some(capabilities) = response
            .as_ref()
            .and_then(|message| message.pointer("/result/capabilities"))
        {
            let encoding = capabilities["positionEncoding"]
                .as_str()
                .unwrap_or("utf-16");
            documents.lock().unwrap().encoding = Encoding::from_lsp(encoding);
        }
        let hover_response = response.and_then(|message| {
            let id = message.get("id")?.to_string();
            let comment = hover_requests.lock().unwrap().remove(&id)?;
            Some((message, comment))
        });

        match hover_response {
            Some((message, comment)) => {
                let hover = hover.clone();
                let client_tx = client_tx.clone();
                tokio::spawn(async move {
                    let message = rewrite_hover(message, comment, &hover).await;
                    let _ = client_tx.send(serde_json::to_vec(&message).unwrap());
                });
            }
            None => {
                let _ = client_tx.send(body);
            }
        }
    }

    // The server closed its stdout: flush what is pending and report its status.
    drop(client_tx);
    let _ = tokio::time::timeout(Duration::from_secs(2), client_writer).await;
    Ok(child.wait().await?)
}

/// Forwards everything from Zed to the server, keeping the open documents up
/// to date and remembering hover requests with the comment under the cursor.
async fn forward_client_messages(
    mut server_stdin: tokio::process::ChildStdin,
    hover_requests: HoverRequests,
    documents: Arc<Mutex<Documents>>,
) {
    let result = async {
        let mut stdin = BufReader::new(tokio::io::stdin());
        while let Some(body) = read_message(&mut stdin).await? {
            if let Ok(message) = serde_json::from_slice::<Value>(&body) {
                let method = message["method"].as_str().unwrap_or_default();
                if method == "textDocument/hover" {
                    if let Some(id) = message.get("id") {
                        log!("hover request {id}");
                        let comment =
                            hovered_comment(&documents.lock().unwrap(), &message["params"]);
                        hover_requests
                            .lock()
                            .unwrap()
                            .insert(id.to_string(), comment);
                    }
                } else if method.starts_with("textDocument/did") {
                    documents.lock().unwrap().apply(&message);
                }
            }
            write_message(&mut server_stdin, &body).await?;
        }
        // Dropping server_stdin here signals EOF to the server.
        anyhow::Ok(())
    }
    .await;
    match result {
        Ok(()) => log!("client closed the connection"),
        Err(error) => log!("forwarding to the server failed: {error:#}"),
    }
}

/// The comment at the position of `textDocument/hover` params, if any.
fn hovered_comment(documents: &Documents, params: &Value) -> Option<HoveredComment> {
    let text = documents.text(params["textDocument"]["uri"].as_str()?)?;
    let comment = find_comment(text, offset(text, &params["position"], documents.encoding))?;
    Some(HoveredComment {
        text: comment.text,
        range: serde_json::json!({
            "start": position(text, comment.start, documents.encoding),
            "end": position(text, comment.end, documents.encoding),
        }),
    })
}

async fn rewrite_hover(
    message: Value,
    comment: Option<HoveredComment>,
    hover: &HoverTranslator,
) -> Value {
    let started = Instant::now();
    let id = message["id"].clone();
    let original = message["result"].clone();
    let message = match comment {
        // The server has nothing to show over a comment (and no error).
        Some(comment) if original.is_null() && message.get("error").is_none() => {
            hover.comment(message, &comment.text, comment.range).await
        }
        _ => hover.augment(message).await,
    };
    let outcome = if message["result"].is_null() {
        "empty"
    } else if message["result"] == original {
        "unchanged"
    } else {
        "rewritten"
    };
    log!("hover response {id}: {outcome} in {:?}", started.elapsed());
    message
}
