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
    arg.push(match val {
        Value::Boolean(toml_bool) => if *toml_bool {return arg} else {return vec![]},
        Value::Integer(toml_int) => toml_int.to_string(),
        Value::String(toml_string) => toml_string.to_owned(),
        Value::Array(arr) => unpack_array_args(arr),
        _ => panic!("Found invalid type in TOML file, values must be one of Bool, String, Integer, or Array"),
    });
    arg
}

fn unpack_array_args(arr: &Vec<Value>) -> String {
    let mut arg = "".to_string();
    for (i, v) in arr.iter().enumerate() {
        arg.push_str(&match v {
            Value::Integer(toml_int) => toml_int.to_string(),
            Value::String(toml_string) => toml_string.to_owned(),
            _ => panic!("Incorrect type found in TOML, array values must be Strings or Integers"),
        });
        if i != arr.len() - 1 {
            arg.push_str(",")
        }
    }
    arg
}
