mod client;
mod diff;
mod nothing;
pub use diff::*;

pub use client::as_token_stream_result;
pub use client::ListArg;
pub use client::MetadataClient;
pub use client::NameSpace;
pub use client::TokenStreamResult;
pub use nothing::DoNothingClient;

pub type SharedClient<C> = std::sync::Arc<C>;
