
use std::pin::Pin;
use std::marker::Unpin;
use std::task::Context;
use std::task::Poll;

use futures::stream::Stream;

use log::trace;
use pin_utils::unsafe_pinned;
use pin_utils::unsafe_unpinned;
use std::mem;


/// Watch Stream suitable for parsing Kubernetes HTTP stream
/// It relies on inner stream which returns streams of bytes
pub struct WatchStream<S>
where
    S: Stream,
{
    stream: S,
    done: bool,
    buffer: Vec<u8>,
}

impl <S>Unpin for WatchStream<S> where S: Stream {}

impl<S> WatchStream<S>
where
    S: Stream<Item = Vec<u8>>
{
    unsafe_pinned!(stream: S);
    unsafe_unpinned!(buffer: Vec<u8>);
    unsafe_unpinned!(done: bool);

    pub fn new(stream: S) -> Self {

        let buffer = Vec::new();
        WatchStream {
            stream,
            done: false,
            buffer
        }
    }
}

const SEPARATOR: u8 = b'\n';

impl<S> Stream for WatchStream<S>
where
    S: Stream<Item = Vec<u8>>
{
    type Item = Vec<u8>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {

        let mut done = self.as_ref().done;
        let mut last_buffer = mem::replace(self.as_mut().buffer(),Vec::new());
        
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
                            Some(mut chunk) => {
                                trace!("got inner stream len: {}",chunk.len());
                               // trace!("chunk: {}", String::from_utf8_lossy(&chunk).to_string());
                                last_buffer.append(&mut chunk);
                            },
                            None => {
                                done = true;
                                break;   
                            }
                        }                        
                    }
                }
            }
        }

        mem::replace(self.as_mut().done(),done);

        if last_buffer.len() > 0 {
            trace!("no more inner, buffer len: {}",last_buffer.len());
           // trace!("chunk: {:#}",String::from_utf8_lossy(&last_buffer).to_string());

            if let Some(i) = last_buffer.iter().position(|&c| c == SEPARATOR) {
                trace!("found separator at: {}", i);
                let remainder = last_buffer.split_off(i+1);                            
                // need to truncate last one since it contains remainder
                last_buffer.truncate(last_buffer.len()-1);
                mem::replace(self.as_mut().buffer(),remainder);
                return Poll::Ready(Some(last_buffer))
            } else {
                trace!("no separator");
                if done {
                    trace!("since we are done, returning last buffer");
                    return Poll::Ready(Some(last_buffer));
                }   
                mem::replace(self.as_mut().buffer(),last_buffer);
                       
            }
        } else {
            trace!("no buffer, swapping pending");
            mem::replace(self.as_mut().buffer(),last_buffer);
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



#[cfg(test)]
mod test {

    use std::io::Error as IoError;

    use futures::stream::StreamExt;
    use isahc::Body;
    use flv_future_core::test_async;
    use crate::stream::BodyStream;

    use super::WatchStream;
    
    #[test_async]
    async fn test_stream() -> Result<(),IoError> {

        let raw_body = "apple\nbanana\ngrape\n";
    
        println!("body: {:#}",raw_body);

        let body = Body::from(raw_body);

        let stream = BodyStream::new(body);

        let mut wstream = WatchStream::new(stream);

        let mut chunks = vec![];
        while let Some(chunk) = wstream.next().await {
            println!("content: {}",chunk.len());
            chunks.push(chunk);
        }
        assert_eq!(chunks.len(),3);
        assert_eq!(chunks[0].len(),5);
        assert_eq!(chunks[1].len(),6);
        Ok(())

    }
}