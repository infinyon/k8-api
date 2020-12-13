mod client;
mod diff;
mod nothing;
pub use diff::*;

pub use client::as_token_stream_result;
pub use client::ListArg;
pub use client::MetadataClient;
pub use client::MetadataClientError;
pub use client::NameSpace;
pub use client::TokenStreamResult;
pub use nothing::DoNothingClient;
pub use nothing::DoNothingError;

pub type SharedClient<C> = std::sync::Arc<C>;

pub use k8_obj_metadata as metadata;
