use std::collections::HashMap;
use std::fs::read_to_string;

use toml::value::Value;

pub fn get_toml_config(toml_path: String) -> Vec<String> {
    let toml_contents = read_to_string(&toml_path).expect(&format!("failed to read toml file at {}", &toml_path));
    let parsed: HashMap<String, Value> =
        toml::from_str(&toml_contents).expect(&format!("failed to parse toml file at {}", &toml_path));
    let mut clap_args: Vec<String> = vec!["armada".to_string()];
    for (key, val) in &parsed {
        clap_args.append(&mut get_flag(key, val));
    }
    clap_args
}

fn get_flag(key: &String, val: &Value) -> Vec<String> {
    let flag = format!("--{}", &key);
    let mut arg = vec![flag];
    match val {
        Value::Boolean(_) => {}
        Value::String(val) => arg.push(val.to_string()),
        Value::Array(vals) => {
            let mut arg_str = "".to_string();
            for (i, v) in vals.iter().enumerate() {
                arg_str.push_str(
                    v.as_str()
                        .expect("Incorrect type found in TOML, array values must be Strings"),
                );
                if i != vals.len() - 1 {
                    arg_str.push_str(",")
                }
            }
            arg.push(arg_str);
        }
        _ => panic!("Incorrect type found in TOML file, values must be one of Bool, String, or String-Array",),
    }
    arg
}
