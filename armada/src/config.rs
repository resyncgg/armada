use std::fs::read_to_string;

use serde::Deserialize;
use toml::value::{Array, Value};

#[derive(Deserialize)]
struct TomlInput {
    listening_port: Option<String>,
    ports: Option<Array>,
    quiet: Option<Value>,
    rate_limit: Option<Value>,
    retries: Option<Value>,
    stream: Option<Value>,
    source_ip: Option<Value>,
    targets: Option<Array>,
    target_file: Option<Value>,
    timeout: Option<Value>,
    top100: Option<Value>,
    top1000: Option<Value>,
}

pub fn get_toml_config(path: String) -> Vec<String> {
    let toml_contents = read_to_string(path).expect("failed to read toml file");
    let parsed: TomlInput = toml::from_str(&toml_contents).expect("failed to parse toml");
    let mut args: Vec<String> = vec!["armada".to_string()];

    get_listening_port(&parsed, &mut args);
    get_ports(&parsed, &mut args);
    get_quiet(&parsed, &mut args);
    get_rate_limit(&parsed, &mut args);
    get_retries(&parsed, &mut args);
    get_stream(&parsed, &mut args);
    get_targets(&parsed, &mut args);
    get_timeout(&parsed, &mut args);

    return args;
}

fn get_listening_port(parsed: &TomlInput, args: &mut Vec<String>) {
    parsed.listening_port.as_ref().map(|port| {
        args.push("--listening-port".to_string());
        args.push(port.to_string())
    });
}

fn get_ports(parsed: &TomlInput, args: &mut Vec<String>) {
    parsed.ports.as_ref().map(|ports| {
        args.push("--ports".to_string());
        let ports_string = ports.iter().map(|x| x.to_string() + ",").collect::<String>();
        args.push(ports_string[0..ports_string.len() - 1].to_string()) // remove trailing comma
    });
    parsed.top100.as_ref().map(|_| args.push("--top100".to_string()));
    parsed.top1000.as_ref().map(|_| args.push("--top1000".to_string()));
}

fn get_quiet(parsed: &TomlInput, args: &mut Vec<String>) {
    parsed.quiet.as_ref().map(|_| args.push("--quiet".to_string()));
}

fn get_rate_limit(parsed: &TomlInput, args: &mut Vec<String>) {
    parsed.rate_limit.as_ref().map(|limit| {
        args.push("--rate-limit".to_string());
        args.push(limit.to_string())
    });
}

fn get_retries(parsed: &TomlInput, args: &mut Vec<String>) {
    parsed.retries.as_ref().map(|retries| {
        args.push("--retries".to_string());
        args.push(retries.to_string())
    });
}

fn get_stream(parsed: &TomlInput, args: &mut Vec<String>) {
    parsed.stream.as_ref().map(|_| args.push("--stream".to_string()));
    parsed.source_ip.as_ref().map(|source_ip| {
        args.push("--source-ip".to_string());
        args.push(source_ip.to_string())
    });
}

fn get_targets(parsed: &TomlInput, args: &mut Vec<String>) {
    parsed.targets.as_ref().map(|targets| {
        args.push("--targets".to_string());
        let t = targets
            .iter()
            .map(|x| x.as_str().expect("invalid type for targets").to_string() + ",")
            .collect::<String>();
        args.push(t[0..t.len() - 1].to_string()) // remove trailing comma
    });
    parsed.target_file.as_ref().map(|file| {
        args.push("--target-file".to_string());
        args.push(file.to_string())
    });
}

fn get_timeout(parsed: &TomlInput, args: &mut Vec<String>) {
    parsed.timeout.as_ref().map(|timeout| {
        args.push("--timeout".to_string());
        args.push(timeout.to_string())
    });
}