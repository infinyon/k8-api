use std::{pin::Pin, task::Poll};

use bytes::{Bytes, BufMut};
use futures_util::{AsyncRead, Stream, StreamExt};

pub struct LogStream(pub Pin<Box<dyn Stream<Item = Bytes> + Send + Sync + 'static>>);

impl AsyncRead for LogStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        mut buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match self.0.poll_next_unpin(cx) {
            Poll::Ready(Some(chunk)) => {
                buf.put_slice(&chunk);
                buf.put_u8(0x0A);
                Poll::Ready(std::io::Result::Ok(chunk.len() + 1))
            }
            Poll::Ready(None) => Poll::Ready(std::io::Result::Ok(0)),
            Poll::Pending => Poll::Pending,
        }
    }
}
