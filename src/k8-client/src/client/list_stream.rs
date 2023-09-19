use std::marker::PhantomData;
use std::mem::replace;
use std::mem::transmute;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use tracing::debug;
use tracing::error;
use tracing::trace;

use futures_util::future::Future;
use futures_util::future::FutureExt;
use futures_util::stream::Stream;
use pin_utils::unsafe_pinned;
use pin_utils::unsafe_unpinned;

use k8_metadata_client::ListArg;
use k8_metadata_client::NameSpace;

use k8_types::{K8List, Spec};
use k8_types::options::ListOptions;
use crate::K8Client;
use crate::SharedK8Client;

type K8ListImpl<'a, S> =
    Option<Pin<Box<dyn Future<Output = anyhow::Result<K8List<S>>> + Send + 'a>>>;

pub struct ListStream<'a, S>
where
    S: Spec,
{
    arg: Option<ListArg>,
    limit: u32,
    done: bool,
    namespace: NameSpace,
    client: SharedK8Client,
    inner: K8ListImpl<'a, S>,
    data1: PhantomData<S>,
}

impl<S> ListStream<'_, S>
where
    S: Spec,
{
    #[allow(unused)]
    pub fn new(
        namespace: NameSpace,
        limit: u32,
        arg: Option<ListArg>,
        client: SharedK8Client,
    ) -> Self {
        Self {
            done: false,
            namespace,
            limit,
            arg,
            client,
            inner: None,
            data1: PhantomData,
        }
    }
}

impl<'a, S> Unpin for ListStream<'a, S> where S: Spec {}

impl<'a, S> ListStream<'a, S>
where
    S: Spec,
{
    unsafe_pinned!(inner: K8ListImpl<'a, S>);
    unsafe_unpinned!(client: SharedK8Client);

    /// given continuation, generate list option
    fn list_option(&self, continu: Option<String>) -> ListOptions {
        let field_selector = match &self.arg {
            None => None,
            Some(arg) => arg.field_selector.clone(),
        };

        let label_selector = match &self.arg {
            None => None,
            Some(arg) => arg.label_selector.clone(),
        };

        ListOptions {
            limit: Some(self.limit),
            continu,
            field_selector,
            label_selector,
            ..Default::default()
        }
    }
}

impl<S> ListStream<'_, S>
where
    S: Spec + 'static,
{
    #[allow(clippy::transmute_ptr_to_ptr)]
    fn set_inner(mut self: Pin<&mut Self>, list_option: Option<ListOptions>) {
        let namespace = self.as_ref().namespace.clone();
        let current_client = &self.as_ref().client;
        // HACK, we transmute the lifetime so that we satisfy borrow checker. should be safe....
        let client: &'_ K8Client =
            unsafe { transmute::<&'_ K8Client, &'_ K8Client>(current_client) };
        self.as_mut()
            .inner()
            .replace(client.retrieve_items_inner(namespace, list_option).boxed());
    }
}

impl<S> Stream for ListStream<'_, S>
where
    S: Spec + 'static,
{
    type Item = K8List<S>;

    fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        trace!(
            "{}: polling, done: {}, inner none: {}",
            S::label(),
            self.as_ref().done,
            self.as_ref().inner.is_none()
        );

        if self.as_ref().done {
            trace!("{} is done, returning none", S::label());
            return Poll::Ready(None);
        }

        if self.as_ref().inner.is_none() {
            trace!("{} no inner set.", S::label());
            let list_option = self.as_ref().list_option(None);
            self.as_mut().set_inner(Some(list_option));
            trace!("{} set inner, returning pending", S::label());
        }

        trace!("{} polling inner", S::label());
        match self.as_mut().inner().as_pin_mut() {
            Some(fut) => match fut.poll(ctx) {
                Poll::Pending => {
                    trace!("{} inner was pending, loop continue", S::label());
                    Poll::Pending
                }
                Poll::Ready(val) => {
                    match val {
                        Ok(list) => {
                            debug!("{} inner returned items: {}", S::label(), list.items.len());
                            // check if we have continue
                            if let Some(_cont) = &list.metadata._continue {
                                debug!("{}: we got continue: {}", S::label(), _cont);
                                let list_option = self.as_ref().list_option(Some(_cont.clone()));
                                self.set_inner(Some(list_option));
                                trace!("{}: ready and set inner, returning ready", S::label());
                            } else {
                                debug!("{} no more continue, marking as done", S::label());
                                // we are done
                                let _ = replace(&mut self.as_mut().done, true);
                            }
                            Poll::Ready(Some(list))
                        }
                        Err(err) => {
                            error!("{}: error in list stream: {}", S::label(), err);
                            let _ = replace(&mut self.as_mut().done, true);
                            Poll::Ready(None)
                        }
                    }
                }
            },
            None => panic!("{} inner should be always set", S::label()),
        }
    }
}
