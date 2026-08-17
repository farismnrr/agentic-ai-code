use super::{LspError, MAX_LSP_HEADER_BYTES, MAX_LSP_MESSAGE_BYTES};
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub(super) async fn read_message<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Value, LspError> {
    let mut header = Vec::with_capacity(256);
    let mut byte = [0u8; 1];
    loop {
        if header.len() >= MAX_LSP_HEADER_BYTES {
            return Err(LspError::MalformedResponse);
        }
        match reader.read_exact(&mut byte).await {
            Ok(_) => header.push(byte[0]),
            Err(error)
                if error.kind() == std::io::ErrorKind::UnexpectedEof && header.is_empty() =>
            {
                return Err(LspError::Crashed);
            }
            Err(_) => return Err(LspError::MalformedResponse),
        }
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header_text = std::str::from_utf8(&header).map_err(|_| LspError::MalformedResponse)?;
    let mut content_length = None;
    for line in header_text[..header_text.len() - 4].split("\r\n") {
        if line.is_empty() {
            continue;
        }
        let Some((name, raw_value)) = line.split_once(':') else {
            return Err(LspError::MalformedResponse);
        };
        if name.eq_ignore_ascii_case("Content-Length") {
            if content_length.is_some() {
                return Err(LspError::MalformedResponse);
            }
            let value = raw_value.trim();
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(LspError::MalformedResponse);
            }
            let parsed = value
                .parse::<usize>()
                .map_err(|_| LspError::MalformedResponse)?;
            if parsed > MAX_LSP_MESSAGE_BYTES {
                return Err(LspError::OversizedResponse);
            }
            content_length = Some(parsed);
        }
    }
    let length = content_length.ok_or(LspError::MalformedResponse)?;
    let mut payload = vec![0u8; length];
    reader
        .read_exact(&mut payload)
        .await
        .map_err(|_| LspError::MalformedResponse)?;
    std::str::from_utf8(&payload).map_err(|_| LspError::MalformedResponse)?;
    serde_json::from_slice(&payload).map_err(|_| LspError::MalformedResponse)
}

pub(super) async fn write_message<W: AsyncWrite + Unpin>(
    writer: &mut W,
    value: &Value,
) -> Result<(), LspError> {
    let payload = serde_json::to_vec(value).map_err(|_| LspError::Internal)?;
    if payload.len() > MAX_LSP_MESSAGE_BYTES {
        return Err(LspError::OversizedResponse);
    }
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    writer
        .write_all(header.as_bytes())
        .await
        .map_err(|_| LspError::Crashed)?;
    writer
        .write_all(&payload)
        .await
        .map_err(|_| LspError::Crashed)?;
    writer.flush().await.map_err(|_| LspError::Crashed)
}
