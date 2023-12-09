use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};

pub fn read_hashmap_from_file<T>(name: &str, parser: fn(String) -> T) -> HashMap<String, T> {
    let mut keymap: HashMap<String, T> = HashMap::new();
    let file = File::open(name).expect("Failed to open file");
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.expect("cannot read line");
        let fields: Vec<&str> = line.trim().split('\t').collect();
        keymap.insert(fields[0].to_string(), parser(fields[1].to_string()));
    }
    keymap
}

pub fn dump_hashmap_to_file(name: &str, hashmap: &HashMap<String, String>) {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&name)
        .expect("Unable to create file");

    let mut writer = BufWriter::new(file);

    for (key, value) in hashmap {
        writeln!(&mut writer, "{}\t{}", key, value).expect("Unable to write to file");
    }
}
