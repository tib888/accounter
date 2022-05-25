pub mod account;
pub mod account_hub;
pub mod actions;
pub mod amount;
pub mod ledger;

use crate::account::*;
use crate::account_hub::*;
use crate::ledger::InMemoryLedger;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::process;

#[macro_use]
extern crate pest_derive;

fn start(filename: &str) {
    if let Ok(file) = File::open(filename) {
        let reader = BufReader::new(file);
        let mut accounts =
            AccountHub::new(|_client_id| Account::new(Box::new(InMemoryLedger::new())));
        accounts.process_csv(reader);
        let _err = accounts.print_summary(&mut std::io::stdout()); //TODO use error
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("ERROR: missing command line argument: the name of transactions file.");
        process::exit(1);
    }
    let filename = &args[1];
    start(filename);
}
