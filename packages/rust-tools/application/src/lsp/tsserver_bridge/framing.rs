//! Bounded stdout framing and the child read loop (Required fix 3) — split
//! out of `tsserver_bridge.rs` to stay under the maintainability
//! file-length budget.

use super::TsServerBridge;
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};

/// Reads one newline-terminated line from `reader`, bounded to `limit`
/// bytes, without ever allocating past that bound: unlike
/// `AsyncBufReadExt::read_line`, which grows its `String` and only checks
/// the size *after* a full line (or the limit-crossing chunk) has already
/// been buffered into it, this walks the reader's own internal buffer via
/// `fill_buf`/`consume` and rejects before any oversized accumulation.
///
/// Returns `Ok(None)` on a clean EOF with no partial line pending (normal
/// child exit), `Ok(Some(line))` for a complete line with the trailing
/// newline stripped, and `Err(())` for any read error, an EOF mid-line, or
/// a line that would exceed `limit`.
pub(super) async fn read_bounded_line<R: tokio::io::AsyncBufRead + Unpin>(
    reader: &mut R,
    limit: usize,
) -> Result<Option<Vec<u8>>, ()> {
    let mut line: Vec<u8> = Vec::new();
    loop {
        let buf = reader.fill_buf().await.map_err(|_| ())?;
        if buf.is_empty() {
            return if line.is_empty() { Ok(None) } else { Err(()) };
        }
        if let Some(newline_at) = buf.iter().position(|byte| *byte == b'\n') {
            if line.len() + newline_at > limit {
                let consumed = newline_at + 1;
                reader.consume(consumed);
                return Err(());
            }
            line.extend_from_slice(&buf[..newline_at]);
            reader.consume(newline_at + 1);
            return Ok(Some(line));
        }
        if line.len() + buf.len() > limit {
            let consumed = buf.len();
            reader.consume(consumed);
            return Err(());
        }
        line.extend_from_slice(buf);
        let consumed = buf.len();
        reader.consume(consumed);
    }
}

/// Drives the sandboxed `tsserver.js` child's stdout: bounded-framed lines
/// in, matching pending `request()` callers answered out. Any read error,
/// oversized/mid-line EOF, or clean child exit is a fatal bridge condition
/// (Required fix 6) — the owning session must not be left silently
/// degraded while still reporting healthy.
pub(super) async fn read_loop(stdout: tokio::process::ChildStdout, bridge: Arc<TsServerBridge>) {
    let mut reader = BufReader::new(stdout);
    loop {
        let line = match read_bounded_line(&mut reader, super::MAX_TSSERVER_LINE_BYTES).await {
            Ok(Some(line)) => line,
            Ok(None) | Err(()) => {
                bridge.mark_fatal().await;
                return;
            }
        };
        let Some(start) = line.iter().position(|byte| !byte.is_ascii_whitespace()) else {
            continue;
        };
        let end = line
            .iter()
            .rposition(|byte| !byte.is_ascii_whitespace())
            .map(|index| index + 1)
            .unwrap_or(line.len());
        let trimmed = &line[start..end];
        let Ok(value) = serde_json::from_slice::<Value>(trimmed) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("response") {
            continue;
        }
        let Some(request_seq) = value.get("request_seq").and_then(Value::as_i64) else {
            continue;
        };
        let body = if value.get("success").and_then(Value::as_bool) == Some(true) {
            value.get("body").cloned().unwrap_or(Value::Null)
        } else {
            Value::Null
        };
        if let Some(sender) = bridge.pending.responses.lock().await.remove(&request_seq) {
            let _ = sender.send(body);
        }
    }
}
