use cidr_utils::cidr::{Ipv4Cidr, Ipv4CidrIpv4AddrIterator};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use crate::armada::config::host::HostIterator;
use crate::armada::config::port::PortIterator;
use itertools::{Itertools, Product};
use lazy_static::lazy_static;

use tokio::sync::mpsc::{
    unbounded_channel,
    UnboundedReceiver,
    UnboundedSender
};

pub enum ArmadaWorkMessage {
    Results(Vec<SocketAddr>),
    Stats {
        total_processed_ports: u128,
        current_inflight_packets: u128,
        total_packets_sent: u128
    }
}

impl ArmadaWorkMessage {
    pub fn results(results: Vec<SocketAddr>) -> ArmadaWorkMessage {
        ArmadaWorkMessage::Results(results)
    }

    pub fn stats(
        total_processed_ports: u128,
        current_inflight_packets: u128,
        total_packets_sent: u128
    ) -> ArmadaWorkMessage {
        ArmadaWorkMessage::Stats {
            total_processed_ports,
            current_inflight_packets,
            total_packets_sent
        }
    }
}

// todo: change to struct to carry work info
pub(crate) struct ArmadaWork {
    pub(crate) remote_addrs: Product<HostIterator, PortIterator>,
    pub(crate) port_retries: u8,
    pub(crate) port_timeout: Duration,
    pub(crate) packets_per_second: Option<usize>,
    pub(crate) source_ipv4_addrs: Vec<Ipv4Addr>,
    pub(crate) source_ipv6_addrs: Vec<Ipv6Addr>,
    pub(crate) reporting_channel: UnboundedSender<ArmadaWorkMessage>,
}

impl ArmadaWork {
    pub(crate) fn new(
        remote_hosts: HostIterator,
        ports: PortIterator,
        port_retries: u8,
        port_timeout: Duration,
        packets_per_second: Option<usize>,
        source_ipv4_addrs: Vec<Ipv4Addr>,
        source_ipv6_addrs: Vec<Ipv6Addr>,
        reporting_channel: UnboundedSender<ArmadaWorkMessage>,
    ) -> Self {
        let remote_addrs = remote_hosts.cartesian_product(ports);

        Self {
            remote_addrs,
            port_retries,
            port_timeout,
            packets_per_second,
            source_ipv4_addrs,
            source_ipv6_addrs,
            reporting_channel,
        }
    }
}
