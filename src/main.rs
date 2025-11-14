use std::{env::args, fs, path::PathBuf};

use crate::inspector::inspect_bytecode;

mod bytecode;
mod errors;
mod inspector;
mod syntax;

fn main() {
    let args: Vec<_> = args().collect();
    let filename: PathBuf = args
        .get(1)
        .expect("Usage: runner <bytecode file>")
        .parse()
        .unwrap();
    let content = fs::read(&filename).expect("Unable to read the file");
    let result = inspect_bytecode(&content)
        .inspect_err(|e| println!("Error while inspecting: {:?}", e))
        .unwrap();
    let mut result_vec: Vec<_> = result.iter().collect();
    result_vec.sort_by_key(|k| (-(*k.1 as i32), k.0));
    for ((fst, snd), count) in result_vec {
        println!(
            "{:<20} -> {:<20}: {}",
            format!("{}", fst),
            format!("{}", snd),
            count
        );
    }
}
