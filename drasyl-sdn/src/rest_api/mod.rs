pub mod auth;
mod client;
mod error;
mod server;
mod status;

pub use auth::*;
pub use client::*;
pub use server::*;
pub use status::*;

use std::net::{Ipv4Addr, SocketAddrV4};
use tracing::error;

pub(crate) const API_LISTEN_DEFAULT: SocketAddrV4 =
    SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 22527);
pub const AUTH_FILE_DEFAULT: &str = "auth.token";
pub(crate) const API_TOKEN_LEN_DEFAULT: usize = 24;
