mod args;
mod ranges;
mod run_variants;
use std::collections::HashMap;
use csv::Writer;
use std::error::Error;

use std::{net::{
    IpAddr,
    Ipv4Addr,
    Ipv6Addr, SocketAddr,
}, io::ErrorKind};

use armada_lib::Armada;

use crate::args::ArmadaCLIConfig;

const DEFAULT_REPORT_NAME: &str = "scan_results.csv";


#[tokio::main]
async fn main() {
    let ArmadaCLIConfig {
        targets,
        quiet_mode,
        ports,
        rate_limit,
        listening_port,
        retries,
        timeout,
        source_ips,
        stream_results,
        generate_report
    } = args::get_armada_cli_config();

    let armada = Armada::new(listening_port);

    let (source_ipv4, source_ipv6) = split_and_enforce_source_ips(source_ips).await;

    let syn_scan_results = if quiet_mode {
        use run_variants::QuietArmada;

        armada
            .run_quiet(targets, ports, source_ipv4, source_ipv6, retries, timeout, rate_limit, stream_results, generate_report)
            .await
    } else {
        use run_variants::ProgressArmada;

        armada
            .run_with_stats(targets, ports, source_ipv4, source_ipv6, retries, timeout, rate_limit, stream_results, generate_report)
            .await
    };


    if !stream_results {
        let scan_results = &syn_scan_results;
        scan_results.into_iter().for_each(|remote| {
            println!("{}:{}", remote.ip(), remote.port());
        });
    }

    if generate_report {
        write_report(&syn_scan_results);
    };
}

fn write_report(scan_results: &Vec<SocketAddr>) -> Result<(), csv::Error> {
    let mut unique_ips = HashMap::new();

    for item in scan_results {
        let ip = item.ip().to_string();
        let port = item.port().to_string();

        let entry = unique_ips.entry(ip).or_insert(Vec::new());
        entry.push(port);
    }

    let mut wtr = Writer::from_path(DEFAULT_REPORT_NAME)?;
    
    wtr.write_record(["Remote IP", "Remote Port"]).iter();
    for (ip, ports) in unique_ips {
        wtr.write_record([ip.as_str(), ports.join(",").as_str()].iter())?;
    }

    Ok(())
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
