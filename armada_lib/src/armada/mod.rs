pub mod config;
mod packet;
mod tcp_ext;
pub mod work;
mod worker;

use tokio::sync::mpsc::{
    unbounded_channel,
    UnboundedReceiver,
    UnboundedSender
};

use crate::armada::config::host::HostIterator;
use crate::armada::config::port::PortIterator;
use crate::armada::work::{ArmadaWork, ArmadaWorkMessage};
use crate::armada::worker::ArmadaWorker;
use futures::stream::StreamExt;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use tokio_stream::wrappers::UnboundedReceiverStream;

/// High performance port scanner
#[derive(Clone)]
pub struct Armada {
    // raw socket
    work_sender: UnboundedSender<ArmadaWork>,
}

impl Armada {
    // todo: add options
    pub fn new(listening_port: u16) -> Self {
        let (work_sender, work_receiver) = unbounded_channel();

        let armada_worker = ArmadaWorker::new(work_receiver);

        std::thread::Builder::new()
            .name("armada_worker".to_string())
            .spawn(move || {
                armada_worker.run(listening_port);
            }).expect("Failed to create armada worker thread.");

        Self { work_sender }
    }

    /// Initiates a port scan and returns the final port scan results.
    pub async fn scan_collect(
        &self,
        remote_hosts: HostIterator,
        ports: PortIterator,
        source_ipv4_addrs: Vec<Ipv4Addr>,
        source_ipv6_addrs: Vec<Ipv6Addr>,
        port_retries: u8,
        port_timeout: Duration,
        packets_per_second: Option<usize>,
    ) -> Vec<SocketAddr> {
        let armada_work_results_handle = self.scan_with_handle(
            remote_hosts,
            ports,
            source_ipv4_addrs,
            source_ipv6_addrs,
            port_retries,
            port_timeout,
            packets_per_second
        );

        // receive all of the reports, filter out non-result messages, and flatten the result list
        UnboundedReceiverStream::new(armada_work_results_handle)
            .filter_map(|report| async move {
                if let ArmadaWorkMessage::Results(results) = report {
                    Some(futures::stream::iter(results))
                } else {
                    None
                }
            })
            .flatten()
            .collect()
            .await
    }

    /// Initiates a port scan and returns a stream handle that can be used to receive both results and statistics of the scan process.
    pub fn scan_with_handle(
        &self,
        remote_hosts: HostIterator,
        ports: PortIterator,
        source_ipv4_addrs: Vec<Ipv4Addr>,
        source_ipv6_addrs: Vec<Ipv6Addr>,
        port_retries: u8,
        port_timeout: Duration,
        packets_per_second: Option<usize>,
    ) -> UnboundedReceiver<ArmadaWorkMessage> {
        let (reporting_channel, report_receiver) = unbounded_channel();

        let work = ArmadaWork::new(
            remote_hosts,
            ports,
            port_retries,
            port_timeout,
            packets_per_second,
            source_ipv4_addrs,
            source_ipv6_addrs,
            reporting_channel,
        );

        self.work_sender.send(work).expect("Failed to send armada work over work sender channel.");

        report_receiver
    }
}
