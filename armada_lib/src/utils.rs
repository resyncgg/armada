use std::net::IpAddr;

/// Attempts to figure out what IP addresses you *probably* want to set as "source" ips for armada
/// It will check for the default route to the internet and grab the IPs configured for that interface.
pub async fn get_default_ips() -> Option<Vec<IpAddr>> {
    get_interface_holding_default_route().await.map(|interface_name| {
        pnet::datalink::interfaces().into_iter()
            .find(|interface| interface.name.eq(&interface_name))
            .map(|interface| {
                interface.ips.into_iter()
                    .map(|networks| networks.ip())
                    .collect::<Vec<_>>()
            })
    }).flatten()
}

/// Fetches the interface that is cited in the default route
/// It's embarrassing for Rust that we don't have an easier to use library for this
async fn get_interface_holding_default_route() -> Option<String> {
    use regex::Regex;
    use tokio::process::Command;

    let ip_output = Command::new("ip")
        .arg("route")
        .arg("show")
        .arg("default")
        .arg("0.0.0.0/0")
        .output()
        .await
        .ok()
        .map(|stdout_bytes| String::from_utf8(stdout_bytes.stdout).ok())
        .flatten()?;

    // parse out device name from `... dev eth0 ...`
    let interface_regex = Regex::new(r"\bdev ([^\s]+)").ok()?;

    interface_regex.captures(&ip_output)
        .map(|captures| captures.get(1))
        .flatten()
        .map(|interface_name| interface_name.as_str().to_string())
}