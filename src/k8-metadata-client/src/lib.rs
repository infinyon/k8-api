mod client;
mod diff;
mod nothing;
mod in_memory;
pub use diff::*;

pub use client::MetadataClient;
pub use client::MetadataClientError;
pub use client::TokenStreamResult;
pub use client::NameSpace;
pub use client::ListArg;
pub use client::as_token_stream_result;
pub use nothing::DoNothingClient;
pub use nothing::DoNothingError;
pub use in_memory::InMemoryClient;
pub use in_memory::InMemoryError;

pub type SharedClient<C> = std::sync::Arc<C>;