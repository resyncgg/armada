use std::net::{
    Ipv4Addr,
    Ipv6Addr,
    SocketAddr,
};
use std::time::Duration;

use armada_lib::{
    Armada,
    HostIterator,
    PortIterator,
};
use async_trait::async_trait;

use crate::run_variants::QuietArmada;

#[async_trait]
impl QuietArmada for Armada {
    async fn run_quiet(
        &self,
        targets: HostIterator,
        ports: PortIterator,
        source_ipv4_addrs: Vec<Ipv4Addr>,
        source_ipv6_addrs: Vec<Ipv6Addr>,
        retries: u8,
        timeout: Duration,
        rate_limit: Option<usize>,
    ) -> Vec<SocketAddr> {
        self.scan_collect(
            targets,
            ports,
            source_ipv4_addrs,
            source_ipv6_addrs,
            retries,
            timeout,
            rate_limit,
        )
        .await
    }
}
