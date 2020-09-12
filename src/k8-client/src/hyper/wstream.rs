use std::marker::Unpin;
use std::mem;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use bytes::BytesMut;
use futures::stream::Stream;
use hyper::body::Bytes;
use hyper::error::Error;
use pin_utils::unsafe_pinned;
use pin_utils::unsafe_unpinned;
use tracing::error;
use tracing::trace;

/// Watch Stream suitable for parsing Kubernetes HTTP stream
/// It relies on inner stream which returns streams of bytes
pub struct WatchStream<S>
where
    S: Stream,
{
    stream: S,
    done: bool,
    buffer: BytesMut,
}

impl<S> Unpin for WatchStream<S> where S: Stream {}

impl<S> WatchStream<S>
where
    S: Stream<Item = Result<Bytes, Error>>,
{
    unsafe_pinned!(stream: S);
    unsafe_unpinned!(buffer: BytesMut);
    unsafe_unpinned!(done: bool);

    pub fn new(stream: S) -> Self {
        let buffer = BytesMut::new();
        WatchStream {
            stream,
            done: false,
            buffer,
        }
    }
}

const SEPARATOR: u8 = b'\n';

impl<S> Stream for WatchStream<S>
where
    S: Stream<Item = Result<Bytes, Error>>,
{
    type Item = Bytes;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut done = self.as_ref().done;
        let mut last_buffer = mem::replace(&mut self.as_mut().buffer, BytesMut::new());

        trace!(
            "entering poll next with buffer: {}, done: {}",
            last_buffer.len(),
            done
        );

        // if not done, we accumulate buffer from inner until they are exhausted
        if !done {
            loop {
                trace!("not done. polling inner");
                match self.as_mut().stream().poll_next(cx) {
                    Poll::Pending => break,
                    Poll::Ready(chunk_item) => {
                        match chunk_item {
                            Some(chunk_result) => {
                                match chunk_result {
                                    Ok(chunk) => {
                                        trace!("got inner stream len: {}", chunk.len());
                                        // trace!("chunk: {}", String::from_utf8_lossy(&chunk).to_string());
                                        last_buffer.extend_from_slice(chunk.as_ref());
                                    }
                                    Err(err) => {
                                        error!("error getting chunk: {}", err);
                                        mem::replace(self.as_mut().done(), true);
                                        return Poll::Ready(None);
                                    }
                                }
                            }
                            None => {
                                done = true;
                                break;
                            }
                        }
                    }
                }
            }
        }

        mem::replace(self.as_mut().done(), done);

        if last_buffer.len() > 0 {
            trace!("no more inner, buffer len: {}", last_buffer.len());
            // trace!("chunk: {:#}",String::from_utf8_lossy(&last_buffer).to_string());

            if let Some(i) = last_buffer.iter().position(|&c| c == SEPARATOR) {
                trace!("found separator at: {}", i);
                let remainder = last_buffer.split_off(i + 1);
                // need to truncate last one since it contains remainder
                last_buffer.truncate(last_buffer.len() - 1);
                mem::replace(&mut self.as_mut().buffer, remainder);
                return Poll::Ready(Some(last_buffer.freeze()));
            } else {
                trace!("no separator");
                if done {
                    trace!("since we are done, returning last buffer");
                    return Poll::Ready(Some(last_buffer.freeze()));
                }
                mem::replace(&mut self.as_mut().buffer, last_buffer);
            }
        } else {
            trace!("no buffer, swapping pending");
            mem::replace(&mut self.as_mut().buffer, last_buffer);
        }

        if done {
            trace!("done, returning none");
            Poll::Ready(None)
        } else {
            trace!("not done, returning pending");
            Poll::Pending
        }
    }
}
