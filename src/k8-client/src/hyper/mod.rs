mod client;
mod config;
mod wstream;

pub use client::K8Client;

pub mod http {
    pub use hyper::Uri;
    pub use ::http::Error;
    pub use ::http::header;
    pub use ::http::status;
}

pub mod prelude {
    pub use hyper::Body;
    pub use hyper::Request;
}