use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

use armada_lib::{
    HostIterator,
    PortIterator,
};
use cidr_utils::cidr::IpCidr;
use clap::{
    crate_version,
    App,
    Arg,
    ArgMatches,
    Values,
};
use rand::Rng;
use atty::Stream;

const DEFAULT_RATE_LIMIT: usize = 10_000; // default rate limit
const DEFAULT_PORT_RETRY: u8 = 2; // default number of additional attempts to make against ports
const DEFAULT_TIMEOUT_IN_MS: u64 = 1_000;

pub(crate) struct ArmadaCLIConfig {
    pub(crate) targets: HostIterator,
    pub(crate) ports: PortIterator,
    pub(crate) quiet_mode: bool,
    pub(crate) rate_limit: Option<usize>,
    pub(crate) listening_port: u16,
    pub(crate) retries: u8,
    pub(crate) timeout: Duration,
    pub(crate) source_ips: Option<Vec<IpAddr>>,
    pub(crate) stream_results: bool
}

pub(crate) fn get_armada_cli_config() -> ArmadaCLIConfig {
    let matches = app_config().get_matches();

    let targets = get_targets(&matches);
    let ports = get_ports(&matches);
    let quiet_mode = get_quiet_mode(&matches);
    let rate_limit = get_rate_limit(&matches);
    let listening_port = get_listening_port(&matches);
    let retries = get_retries(&matches);
    let timeout = get_timeout(&matches);
    let source_ips = get_source_ip_addresses(&matches);
    let stream_results = get_stream_results(&matches);

    if stream_results {
        if !quiet_mode && atty::is(Stream::Stdout) {
            panic!("Streaming only enabled when in quiet mode or when piping results out from armada.");
        }
    }

    ArmadaCLIConfig {
        targets,
        ports,
        quiet_mode,
        rate_limit,
        listening_port,
        retries,
        timeout,
        source_ips,
        stream_results
    }
}

fn get_targets(matches: &ArgMatches) -> HostIterator {
    matches
        .values_of("targets")
        .expect("Targets are required for armada to run.")
        .fold(HostIterator::new(), |host_iterator, target_str| {
            if let Ok(ip_addr) = IpAddr::from_str(target_str) {
                host_iterator.add_ip(ip_addr)
            } else {
                // we'll force this to parse. If it fails, then an illegal value was placed into the target list and we should panic here.
                let cidr = IpCidr::from_str(target_str).expect(&format!("Unable to parse target '{}'.", target_str));

                host_iterator.add_cidr(cidr)
            }
        })
}

fn get_ports(matches: &ArgMatches) -> PortIterator {
    use regex::{
        Match,
        Regex,
    };

    use crate::ranges::{
        TOP_100,
        TOP_1000,
    };

    let user_port_string = matches.values_of("ports");
    let top_100_flag = matches.is_present("top100");
    let top_1000_flag = matches.is_present("top1000");

    let port_strings: Vec<String> = match (user_port_string, top_100_flag, top_1000_flag) {
        (Some(values), ..) => values.map(|value| value.to_string()).collect(),
        (_, true, _) => TOP_100.split(",").map(|def| def.to_string()).collect(),
        (_, _, true) => TOP_1000.split(",").map(|def| def.to_string()).collect(),
        _ => panic!("Ports are required to be supplied for armada to run."),
    };

    let port_regex = Regex::new(r"^(\d+)(?:-(\d+))?$").unwrap();

    port_strings
        .into_iter()
        .fold(PortIterator::new(), |port_iterator, port_str| {
            let capture = port_regex
                .captures(&port_str)
                .expect(&format!("Failed to interpret port flag with value '{}'.", port_str));

            let start_port = capture.get(1).map(|m| m.as_str()).map(|port_str| {
                port_str
                    .parse::<u16>()
                    .expect(&format!("Failed to parse port '{}' into int.", port_str))
            });

            let end_port = capture.get(2).map(|m| m.as_str()).map(|port_str| {
                port_str
                    .parse::<u16>()
                    .expect(&format!("Failed to parse port '{}' into int.", port_str))
            });

            match (start_port, end_port) {
                (Some(start_port), Some(end_port)) => port_iterator.add_range(start_port, end_port),
                (Some(port), None) => port_iterator.add_port(port),
                _ => panic!("Failed to interpret port flag with value '{}'.", port_str),
            }
        })
}

fn get_quiet_mode(matches: &ArgMatches) -> bool { matches.is_present("quiet") }

fn get_rate_limit(matches: &ArgMatches) -> Option<usize> {
    let rate_limit = matches.value_of("rate_limit")
        .map(|value| {
            value
                .parse::<usize>()
                .expect("Rate limit must be a non-negative number.")
        });

    match rate_limit {
        _ if matches.is_present("sanic") => None,
        Some(rate) if rate == 0 => None,
        Some(rate) => Some(rate),
        None => Some(DEFAULT_RATE_LIMIT),
    }
}

fn get_listening_port(matches: &ArgMatches) -> u16 {
    matches
        .value_of("listening_port")
        .map(|value| {
            value
                .parse::<u16>()
                .expect(&format!("Unable to parse listening port value '{}'.", value))
        })
        .unwrap_or_else(|| rand::thread_rng().gen_range(50_000 .. 60_000))
}

fn get_retries(matches: &ArgMatches) -> u8 {
    matches
        .value_of("retries")
        .map(|value| {
            value
                .parse::<u8>()
                .expect(&format!("Unable to parse port retry value '{}'.", value))
        })
        .or(matches.is_present("sanic").then(|| 0))
        .unwrap_or(DEFAULT_PORT_RETRY)
}

fn get_timeout(matches: &ArgMatches) -> Duration {
    let timeout = matches
        .value_of("timeout")
        .map(|value| {
            value
                .parse::<u64>()
                .expect(&format!("Unable to parse timeout value '{}'.", value))
        })
        .unwrap_or(DEFAULT_TIMEOUT_IN_MS);

    Duration::from_millis(timeout)
}

fn get_source_ip_addresses(matches: &ArgMatches) -> Option<Vec<IpAddr>> {
    matches.values_of("source_ips").map(|values| {
        values
            .map(|value| IpAddr::from_str(value).expect(&format!("Unable to parse source IP address '{}'.", value)))
            .collect()
    })
}

fn get_stream_results(matches: &ArgMatches) -> bool {
    matches.is_present("stream")
}

fn app_config() -> App<'static, 'static> {
    App::new("armada")
        .author("d0nut <d0nut@resync.gg>")
        .about("High performance TCP SYN port scanner")
        .version(crate_version!())
        .arg(Arg::with_name("targets")
            .help("The IP and CIDR ranges to scan.")
            .index(1)
            .multiple(true)
            .takes_value(true)
            .require_delimiter(true)
            .value_delimiter(",")
            .required(true))
        .arg(Arg::with_name("ports")
            .help("Sets which ports to scan.")
            .short("p")
            .long("ports")
            .multiple(true)
            .takes_value(true)
            .require_delimiter(true)
            .value_delimiter(",")
            .conflicts_with_all(&["top100", "top1000"])
            .required_unless_one(&["top100", "top1000"]))
        .arg(Arg::with_name("quiet")
            .help("Disables any progress reporting during the scan.")
            .short("q")
            .long("quiet")
            .takes_value(false))
        .arg(Arg::with_name("rate_limit")
            .help("Sets the maximum packets per second. \
            If this is explicitly set to 0, we'll run with no maximum. \
            Defaults to 10kpps. Keep in mind that faster != better.")
            .long("rate-limit")
            .takes_value(true))
        .arg(Arg::with_name("listening_port")
            .help("Sets the port to listen on. If unset, armada will pick a random port from 50000-60000.")
            .long("listening-port")
            .takes_value(true))
        .arg(Arg::with_name("retries")
            .help("Sets the number of additional attempts aramada will take to verify that a port is open. Setting this to '0' will result in ports only being checked once. Defaults to 2.")
            .long("retries")
            .takes_value(true))
        .arg(Arg::with_name("timeout")
            .help("Sets the amount of time, in milliseconds, waited until a sent packet is determined to have been timed out. Defaults to 1 second.")
            .long("timeout")
            .takes_value(true))
        .arg(Arg::with_name("source_ips")
            .help("Adds an ip address (v4 or v6) that armada should use when creating TCP packets. If not set, it will try to use sensible defaults.")
            .long("source-ip")
            .multiple(true)
            .takes_value(true))
        .arg(Arg::with_name("top100")
            .help("Scans for the top 100 most common ports.")
            .long("top100")
            .conflicts_with("top1000")
            .takes_value(false))
        .arg(Arg::with_name("top1000")
            .help("Scans for the top 1,000 most common ports.")
            .long("top1000")
            .takes_value(false))
        .arg(Arg::with_name("stream")
            .help("Enable streaming the results into stdout as they come in. Only works if piping the results out or if quiet mode is enabled.")
            .long("stream")
            .short("s"))
        .arg(Arg::with_name("sanic")
            .hidden(true)
            .long("sanic")
            .takes_value(false))
}
