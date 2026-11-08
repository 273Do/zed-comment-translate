//! Relaying LSP messages between Zed and the wrapped server, rewriting only
//! `textDocument/hover` responses.

pub mod server;
mod transport;

use std::{
    collections::HashSet,
    process::ExitStatus,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::{io::BufReader, process::Child, sync::mpsc};

use crate::hover::HoverTranslator;
use transport::{read_message, write_message};

/// Relays stdin/stdout to the server until it exits, and returns its status.
pub async fn run(mut child: Child, hover: Arc<HoverTranslator>) -> Result<ExitStatus> {
    let server_stdin = child.stdin.take().context("no server stdin")?;
    let mut server_stdout = BufReader::new(child.stdout.take().context("no server stdout")?);
    // Ids of in-flight hover requests, keyed by their JSON representation.
    let hover_ids = Arc::new(Mutex::new(HashSet::<String>::new()));

    // All writes to Zed go through one task so that frames never interleave.
    let (client_tx, mut client_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let client_writer = tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(body) = client_rx.recv().await {
            write_message(&mut stdout, &body).await?;
        }
        anyhow::Ok(())
    });

    tokio::spawn(forward_client_messages(server_stdin, hover_ids.clone()));

    while let Some(body) = read_message(&mut server_stdout).await? {
        let hover_response = serde_json::from_slice::<Value>(&body)
            .ok()
            .filter(|message| message.get("method").is_none())
            .filter(|message| {
                message
                    .get("id")
                    .is_some_and(|id| hover_ids.lock().unwrap().remove(&id.to_string()))
            });

        match hover_response {
            Some(message) => {
                let hover = hover.clone();
                let client_tx = client_tx.clone();
                tokio::spawn(async move {
                    let message = rewrite_hover(message, &hover).await;
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

/// Forwards everything from Zed to the server, remembering hover request ids.
async fn forward_client_messages(
    mut server_stdin: tokio::process::ChildStdin,
    hover_ids: Arc<Mutex<HashSet<String>>>,
) {
    let result = async {
        let mut stdin = BufReader::new(tokio::io::stdin());
        while let Some(body) = read_message(&mut stdin).await? {
            if let Ok(message) = serde_json::from_slice::<Value>(&body) {
                if message["method"] == "textDocument/hover" {
                    if let Some(id) = message.get("id") {
                        log!("hover request {id}");
                        hover_ids.lock().unwrap().insert(id.to_string());
                    }
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

async fn rewrite_hover(message: Value, hover: &HoverTranslator) -> Value {
    let started = Instant::now();
    let id = message["id"].clone();
    let original = message["result"].clone();
    let message = hover.augment(message).await;
    let outcome = if original.is_null() {
        "empty"
    } else if message["result"] == original {
        "unchanged"
    } else {
        "rewritten"
    };
    log!("hover response {id}: {outcome} in {:?}", started.elapsed());
    message
}
