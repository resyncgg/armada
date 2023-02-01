use std::net::{
    Ipv4Addr,
    Ipv6Addr,
    SocketAddr,
};
use std::time::Duration;

use armada_lib::{Armada, HostIterator, PortIterator, ArmadaWorkMessage};
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
        stream_results: bool,
        generate_report: bool
    ) -> Vec<SocketAddr> {
        if stream_results {
            let mut reporting_handle = self.scan_with_handle(
                targets,
                ports,
                source_ipv4_addrs,
                source_ipv6_addrs,
                retries,
                timeout,
                rate_limit,
            );

            while let Some(message) = reporting_handle.recv().await {
                match message {
                    ArmadaWorkMessage::Results(results) => {
                        results.iter().for_each(|remote| {
                            println!("{}:{}", remote.ip(), remote.port());
                        });
                    }
                    _ => {}
                }
            }

            vec![]
        } else {
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
}
