//! LSP base protocol framing (`Content-Length` headers).

use anyhow::{Context, Result};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Reads one message body, or `None` at end of stream.
pub async fn read_message<R: AsyncBufRead + Unpin>(reader: &mut R) -> Result<Option<Vec<u8>>> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await? == 0 {
            return Ok(None);
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = Some(value.trim().parse::<usize>()?);
            }
        }
    }
    let mut body = vec![0; content_length.context("missing Content-Length header")?];
    reader.read_exact(&mut body).await?;
    Ok(Some(body))
}

pub async fn write_message<W: AsyncWrite + Unpin>(writer: &mut W, body: &[u8]) -> Result<()> {
    writer
        .write_all(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes())
        .await?;
    writer.write_all(body).await?;
    writer.flush().await?;
    Ok(())
}
