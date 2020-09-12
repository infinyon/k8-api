mod client;
mod config;
mod stream;
mod wstream;

pub use client::K8Client;
pub mod http {
    pub use isahc::http::*;
}

pub mod prelude {
    pub use isahc::prelude::*;
}
