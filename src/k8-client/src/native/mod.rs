mod client;
mod wstream;
mod config;
mod stream;


pub use client::K8Client;
pub mod http {
    pub use isahc::http::*;
}

pub mod prelude {
    pub use isahc::prelude::*;
}
