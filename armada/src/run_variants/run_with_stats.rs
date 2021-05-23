use std::net::{
    Ipv4Addr,
    Ipv6Addr,
    SocketAddr,
};
use std::time::Duration;

use armada_lib::{
    Armada,
    ArmadaWorkMessage,
    HostIterator,
    PortIterator,
};
use async_trait::async_trait;
use indicatif::{
    MultiProgress,
    ProgressBar,
    ProgressStyle,
};

use crate::run_variants::ProgressArmada;

#[async_trait]
impl ProgressArmada for Armada {
    async fn run_with_stats(
        &self,
        targets: HostIterator,
        ports: PortIterator,
        source_ipv4_addrs: Vec<Ipv4Addr>,
        source_ipv6_addrs: Vec<Ipv6Addr>,
        retries: u8,
        timeout: Duration,
        rate_limit: Option<usize>,
    ) -> Vec<SocketAddr> {
        let mut total_open_ports = Vec::new();
        let total_ports: u128 = targets.size() * ports.size() as u128;
        let _total_packets = total_ports * (1 + retries) as u128;

        let multi_pb = MultiProgress::new();

        let found_and_stats_progress_bar = multi_pb.add(ProgressBar::new_spinner());
        found_and_stats_progress_bar.set_message("0");
        found_and_stats_progress_bar
            .set_style(ProgressStyle::default_spinner().template("{spinner:.yellow} Found: {msg:.green}"));
        found_and_stats_progress_bar.enable_steady_tick(50);

        let inflight_progress_bar = multi_pb.add(ProgressBar::new_spinner());
        inflight_progress_bar.set_message("0");
        inflight_progress_bar
            .set_style(ProgressStyle::default_spinner().template("{spinner:.yellow} In-flight Packets: {msg:.blue}"));
        inflight_progress_bar.enable_steady_tick(50);

        let total_scan_progress_bar = multi_pb.add(ProgressBar::new(total_ports as u64));
        total_scan_progress_bar.set_message("0");
        total_scan_progress_bar.set_style(ProgressStyle::default_bar().template(
            "[ETA: {eta_precise:.dim}] {wide_bar:.red} {pos:.dim}/{len:.dim} ports (Elapsed: {elapsed_precise:.dim})",
        ));
        total_scan_progress_bar.enable_steady_tick(50);

        let mpb_thread_handle = std::thread::spawn(move || multi_pb.join_and_clear());

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
                    total_open_ports.extend(results);
                    found_and_stats_progress_bar.set_message(&format!("{}", total_open_ports.len()));
                }
                ArmadaWorkMessage::Stats {
                    total_processed_ports,
                    current_inflight_packets,
                    total_packets_sent,
                } => {
                    inflight_progress_bar.set_message(&format!("{}", current_inflight_packets));
                    total_scan_progress_bar.set_position((total_packets_sent / (1 + retries) as u128) as u64);
                }
            }
        }

        total_scan_progress_bar.finish_and_clear();
        found_and_stats_progress_bar.finish_and_clear();
        inflight_progress_bar.finish_and_clear();

        mpb_thread_handle.join();

        total_open_ports
    }
}
