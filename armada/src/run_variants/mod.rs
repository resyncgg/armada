mod run_quiet;
mod run_with_stats;

use std::net::{
    Ipv4Addr,
    Ipv6Addr,
    SocketAddr,
};
use std::time::Duration;

use armada_lib::{
    HostIterator,
    PortIterator,
};
use async_trait::async_trait;

#[async_trait]
pub(crate) trait QuietArmada {
    async fn run_quiet(
        &self,
        targets: HostIterator,
        ports: PortIterator,
        source_ipv4_addrs: Vec<Ipv4Addr>,
        source_ipv6_addrs: Vec<Ipv6Addr>,
        retries: u8,
        timeout: Duration,
        rate_limit: Option<usize>,
    ) -> Vec<SocketAddr>;
}

#[async_trait]
pub(crate) trait ProgressArmada {
    async fn run_with_stats(
        &self,
        targets: HostIterator,
        ports: PortIterator,
        source_ipv4_addrs: Vec<Ipv4Addr>,
        source_ipv6_addrs: Vec<Ipv6Addr>,
        retries: u8,
        timeout: Duration,
        rate_limit: Option<usize>,
    ) -> Vec<SocketAddr>;
}
