mod client;
mod config_native;
mod wstream;
mod list_stream;

pub use client::K8Client;
use config_native::*;
use list_stream::*;

pub mod http {
    pub use ::http::header;
    pub use ::http::status;
    pub use ::http::Error;
    pub use hyper::Uri;
}

pub mod prelude {
    pub use hyper::Body;
    pub use hyper::Request;
}
