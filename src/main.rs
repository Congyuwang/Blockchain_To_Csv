use bitcoin_explorer::{Address, BitcoinDB, SConnectedBlock};
use indicatif;
use indicatif::ProgressStyle;
use log::{info, warn};
use simple_logger::SimpleLogger;
use std::fs;
use std::fs::File;
use std::io::{stdin, BufWriter, Write};
use std::path::Path;

///
/// Address to string. Rule:
/// 1. if no address: empty string
/// 2. if one address: address string
/// 3. if more than one address: sort alphabetically and concatenate by '-'
///
#[inline]
fn addresses_to_string(addresses: Box<[Address]>) -> String {
    match addresses.len() {
        0 => String::new(),
        1 => addresses.get(0).unwrap().to_string(),
        _ => {
            let mut addresses: Vec<String> = addresses.into_iter().map(|a| a.to_string()).collect();
            // sort addresses
            addresses.sort();
            addresses.join("-")
        }
    }
}

fn main() {
    // start logger
    SimpleLogger::new().init().unwrap();

    // launchDB
    let db = loop {
        println!("enter path to bitcoin directory (--datadir):");
        let mut db_path = String::new();
        stdin()
            .read_line(&mut db_path)
            .expect("failed to read user input");
        let db_path = Path::new(db_path.trim());
        if !db_path.exists() {
            warn!("bitcoin path: {} not found", db_path.display());
        }
        match BitcoinDB::new(db_path, false) {
            Ok(db) => {
                break db;
            }
            Err(_) => continue,
        }
    };

    info!("launching DB finished");

    // create output path
    println!("enter a directory as output folder (use absolute path!):");
    let mut out_dir = String::new();
    stdin()
        .read_line(&mut out_dir)
        .expect("failed to read user input");
    let out_dir = loop {
        let out_dir = Path::new(out_dir.trim());
        if !out_dir.exists() {
            match fs::create_dir_all(out_dir) {
                Ok(_) => {
                    break out_dir;
                }
                Err(_) => continue,
            }
        }
    };

    // create output files
    let outputs = File::create(out_dir.join("output.csv")).expect("failed to create output.csv");
    let inputs = File::create(out_dir.join("input.csv")).expect("failed to create input.csv");
    let table_header = "timestamp,address,value\n";
    let mut out_writer = BufWriter::new(outputs);
    let mut input_writer = BufWriter::new(inputs);

    // write header
    write!(out_writer, "{}\n", table_header).expect("failed to write header to output.csv");
    write!(input_writer, "{}\n", table_header).expect("failed to write header to input.csv");

    // preparing progress bar
    // compute the total number of transactions (for displaying progress)
    let end = db.get_max_height() as usize;
    let total_number_of_transactions = (0..end)
        .map(|i| db.get_header(i).unwrap().n_tx)
        .sum::<u32>() as u64;
    let bar = indicatif::ProgressBar::new(total_number_of_transactions);
    bar.set_style(ProgressStyle::default_bar().progress_chars("=>-").template(
        "[{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos:>10}/{len:10} tx ({per_sec}, {eta})",
    ));

    // iterate over all blocks
    for blk in db.iter_connected_block::<SConnectedBlock>(end as u32) {
        let time_stamp = blk.header.time;
        // update progress bar
        let len = blk.txdata.len();
        for tx in blk.txdata {
            for input in tx.input {
                let address = addresses_to_string(input.addresses);
                let value = input.value;
                write!(input_writer, "{},{},{}\n", time_stamp, address, value)
                    .expect("failed to write to input.csv");
            }
            for output in tx.output {
                let address = addresses_to_string(output.addresses);
                let value = output.value;
                write!(out_writer, "{},{},{}\n", time_stamp, address, value)
                    .expect("failed to write to output.csv");
            }
        }
        bar.inc(len as u64)
    }

    // finish writing the remaining
    out_writer.flush().expect("failed to flush to output.csv");
    input_writer.flush().expect("failed to flush to input.csv");
    bar.finish();
    info!("job finished");
}
