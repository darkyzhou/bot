use std::collections::HashMap;

pub fn serialize_hashmap(map: &HashMap<String, String>) -> String {
    let mut items: Vec<(&String, &String)> = map.iter().collect();
    items.sort();
    items.iter().fold("".to_string(), |acc, (key, val)| format!("{}\n{}：{}", acc, key, val))
}