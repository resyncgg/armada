use std::collections::HashMap;
use std::fs::read_to_string;
use toml::value::Value;

pub fn get_toml_config(path: String) -> Vec<String> {
    let toml_contents = read_to_string(path).expect("failed to read toml file");
    let parsed: HashMap<String, Value> = toml::from_str(&toml_contents).expect("failed to parse toml");
    let mut args: Vec<String> = vec!["armada".to_string()];
    for (k, v) in &parsed { 
        args.append(&mut get_flag(k, v));
    }
    args
}

fn get_flag(key: &String, val: &Value) -> Vec<String> {
    let mut flag = "--".to_string();
    flag.push_str(&key);
    let mut arg = vec![flag];
    match val {
        Value::Boolean(_) => {},
        Value::String(val) => arg.push(val.to_string()),
        Value::Array(vals) => {
            let mut arg_str = "".to_string();
            for (i, v) in vals.iter().enumerate() {
                arg_str.push_str(v.as_str().unwrap());
                if i != vals.len() - 1 {
                    arg_str.push_str(",")
                }
            }
            arg.push(arg_str);
        }
        _ => panic!(),
    }
    arg
}
