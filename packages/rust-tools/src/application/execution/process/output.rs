use super::now_ms;
use std::io;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;

pub(in crate::application::execution) struct OutputBuffer {
    pub(in crate::application::execution) bytes: Vec<u8>,
    pub(in crate::application::execution) omitted: u64,
    pub(in crate::application::execution) updated_at: u128,
}

impl OutputBuffer {
    pub(in crate::application::execution) fn new(updated_at: u128) -> Self {
        Self {
            bytes: Vec::new(),
            omitted: 0,
            updated_at,
        }
    }

    pub(super) fn push(&mut self, chunk: &[u8], limit: usize) {
        self.updated_at = now_ms();
        if chunk.len() >= limit {
            self.omitted += (self.bytes.len() + chunk.len() - limit) as u64;
            self.bytes = chunk[chunk.len() - limit..].to_vec();
            return;
        }
        self.bytes.extend_from_slice(chunk);
        if self.bytes.len() > limit {
            let drop_count = self.bytes.len() - limit;
            self.bytes.drain(..drop_count);
            self.omitted += drop_count as u64;
        }
    }
}

pub(in crate::application::execution) async fn drain_pipe<R: tokio::io::AsyncRead + Unpin>(
    mut pipe: R,
    output: Arc<Mutex<OutputBuffer>>,
    limit: usize,
) -> Result<(), io::Error> {
    let mut buf = [0u8; 8192];
    loop {
        match pipe.read(&mut buf).await {
            Ok(0) => return Ok(()),
            Err(error) => return Err(error),
            Ok(n) => output.lock().await.push(&buf[..n], limit),
        }
    }
}
