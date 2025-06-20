pub(crate) mod config;

use arc_swap::ArcSwap;
pub use config::*;
use drasyl::node::SendHandle;
use ipnet::Ipv4Net;
use ipnet_trie::IpnetTrie;
use serde::Deserialize;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;
use tun_rs::AsyncDevice as TunDevice;

#[derive(Clone)]
pub struct TunState {
    pub(crate) cancellation_token: CancellationToken,
    pub(crate) device: Arc<TunDevice>,
}

#[derive(PartialEq, Clone)]
pub struct LocalNodeState {
    pub(crate) subnet: Ipv4Net,
    pub(crate) ip: Ipv4Addr,
    pub(crate) access_rules: EffectiveAccessRuleList,
    pub(crate) routes: EffectiveRoutingList,
    pub(crate) hostnames: HashMap<Ipv4Addr, String>,
}

impl LocalNodeState {
    pub(crate) fn tun_state(&self) -> (Ipv4Net, Ipv4Addr) {
        (self.subnet, self.ip)
    }
}

pub struct NetworkInner {
    pub(crate) trie_tx: ArcSwap<IpnetTrie<IpnetTrie<Arc<SendHandle>>>>,
    pub(crate) tun_state: Mutex<Option<TunState>>,
}

impl Default for NetworkInner {
    fn default() -> Self {
        Self {
            trie_tx: ArcSwap::from_pointee(IpnetTrie::new()),
            tun_state: Default::default(),
        }
    }
}

#[derive(Deserialize)]
pub struct Network {
    pub(crate) config_url: String,
    #[serde(skip_deserializing, default)]
    pub(crate) state: Mutex<Option<LocalNodeState>>,
    #[serde(skip_deserializing, default)]
    pub(crate) inner: Arc<NetworkInner>,
}
