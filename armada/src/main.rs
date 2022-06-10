mod args;
mod ranges;
mod run_variants;

use std::net::{
    IpAddr,
    Ipv4Addr,
    Ipv6Addr,
};

use armada_lib::Armada;

use crate::args::ArmadaCLIConfig;

#[tokio::main]
async fn main() {
    let ArmadaCLIConfig {
        targets,
        ports,
        quiet_mode,
        rate_limit,
        listening_port,
        retries,
        timeout,
        source_ips,
        stream_results
    } = args::get_armada_cli_config();

    let armada = Armada::new(listening_port);

    let (source_ipv4, source_ipv6) = split_and_enforce_source_ips(source_ips).await;

    let mut syn_scan_results = if quiet_mode {
        use run_variants::QuietArmada;

        armada
            .run_quiet(targets, ports, source_ipv4, source_ipv6, retries, timeout, rate_limit, stream_results)
            .await
    } else {
        use run_variants::ProgressArmada;

        armada
            .run_with_stats(targets, ports, source_ipv4, source_ipv6, retries, timeout, rate_limit, stream_results)
            .await
    };

    if !stream_results {
        syn_scan_results.sort();

        syn_scan_results.into_iter().for_each(|remote| {
            println!("{}:{}", remote.ip(), remote.port());
        });
    }
}

async fn split_and_enforce_source_ips(source_ips: Option<Vec<IpAddr>>) -> (Vec<Ipv4Addr>, Vec<Ipv6Addr>) {
    // we need to try to
    let source_ips = match source_ips {
        Some(source_ips) => source_ips,
        _ => armada_lib::utils::get_default_ips()
            .await
            .expect("Unable to identify source ip addresses automatically. Please supply them via --source-ip."),
    };

    let (ipv4_addrs, ipv6_addrs): (Vec<_>, Vec<_>) = source_ips.into_iter().partition(|ip_addr| ip_addr.is_ipv4());

    let source_ipv4_addrs: Vec<_> = ipv4_addrs
        .into_iter()
        .map(|ip| match ip {
            IpAddr::V4(ip_v4) => ip_v4,
            _ => unreachable!("Should already be split."),
        })
        .collect();

    let source_ipv6_addrs: Vec<_> = ipv6_addrs
        .into_iter()
        .map(|ip| match ip {
            IpAddr::V6(ip_v6) => ip_v6,
            _ => unreachable!("Should already be split."),
        })
        .collect();

    (source_ipv4_addrs, source_ipv6_addrs)
}
