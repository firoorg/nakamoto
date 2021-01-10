//! Stand-alone light-client daemon. Runs the light-client as a background process.
#![deny(missing_docs, unsafe_code)]

use std::net;
use std::time;

pub use nakamoto_client::client::{Client, Config, Network};
pub use nakamoto_client::error::Error;

pub mod logger;

/// The network reactor we're going to use.
type Reactor = nakamoto_net_poll::Reactor<net::TcpStream>;

/// Run the light-client. Takes an initial list of peers to connect to, a list of listen addresses
/// and the Bitcoin network to connect to.
pub fn run(
    connect: &[net::SocketAddr],
    listen: &[net::SocketAddr],
    network: Network,
) -> Result<(), Error> {
    let mut cfg = Config {
        network,
        listen: if listen.is_empty() {
            vec![([0, 0, 0, 0], 0).into()]
        } else {
            listen.to_vec()
        },
        connect: connect.to_vec(),
        timeout: time::Duration::from_secs(30),
        ..Config::default()
    };
    if !connect.is_empty() {
        cfg.target_outbound_peers = connect.len();
    }

    Client::<Reactor>::new(cfg)?.run()
}
