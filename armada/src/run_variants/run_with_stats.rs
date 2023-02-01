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

const UPDATE_INTERVAL: Duration = Duration::from_millis(50);

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
        stream_output: bool,
        generate_report: bool
    ) -> Vec<SocketAddr> {
        let mut total_open_ports = Vec::new();
        let total_ports: u128 = targets.size() * ports.size() as u128;
        let _total_packets = total_ports * (1 + retries) as u128;

        let multi_pb = MultiProgress::new();

        let found_and_stats_progress_bar = multi_pb.add(ProgressBar::new_spinner());
        found_and_stats_progress_bar.set_message("0");
        found_and_stats_progress_bar
            .set_style(ProgressStyle::default_spinner().template("{spinner:.yellow} Found: {msg:.green}").expect("invalid template"));
        found_and_stats_progress_bar.enable_steady_tick(UPDATE_INTERVAL);

        let inflight_progress_bar = multi_pb.add(ProgressBar::new_spinner());
        inflight_progress_bar.set_message("0");
        inflight_progress_bar
            .set_style(ProgressStyle::default_spinner().template("{spinner:.yellow} In-flight Packets: {msg:.blue}").expect("invalid template"));
        inflight_progress_bar.enable_steady_tick(UPDATE_INTERVAL);

        let total_scan_progress_bar = multi_pb.add(ProgressBar::new(total_ports as u64));
        total_scan_progress_bar.set_message("0");
        total_scan_progress_bar.set_style(ProgressStyle::default_bar().template(get_progress_stylization(&rate_limit, retries)).expect("invalid template"));
        total_scan_progress_bar.enable_steady_tick(UPDATE_INTERVAL);


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
                    if stream_output {
                        results.iter().for_each(|remote| {
                            println!("{}:{}", remote.ip(), remote.port());
                        });
                    }

                    total_open_ports.extend(results);

                    found_and_stats_progress_bar.set_message(format!("{}", total_open_ports.len()));
                }
                ArmadaWorkMessage::Stats {
                    total_processed_ports: _,
                    current_inflight_packets,
                    total_packets_sent,
                } => {
                    inflight_progress_bar.set_message(format!("{}", current_inflight_packets));
                    total_scan_progress_bar.set_position((total_packets_sent / (1 + retries) as u128) as u64);
                }
            }
        }

        total_scan_progress_bar.finish_and_clear();
        found_and_stats_progress_bar.finish_and_clear();
        inflight_progress_bar.finish_and_clear();

        total_open_ports
    }
}

fn get_progress_stylization(rate_limit: &Option<usize>, retries: u8) -> &'static str {
    match (rate_limit, retries) {
        (None, 0) => "[ETA: {eta_precise:.dim}] {wide_bar:.blue} {pos:.dim}/{len:.dim} ports (Elapsed: {elapsed_precise:.dim})",
        _ => "[ETA: {eta_precise:.dim}] {wide_bar:.red} {pos:.dim}/{len:.dim} ports (Elapsed: {elapsed_precise:.dim})"
    }
}
