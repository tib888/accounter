pub mod account;
pub mod account_hub;
pub mod actions;
pub mod amount;
pub mod ledger;

use crate::account::*;
use crate::account_hub::*;
use crate::ledger::InMemoryLedger;

use std::env;
use std::process;

use std::fs::File;
use std::io::BufReader;

#[macro_use]
extern crate pest_derive;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        #[cfg(feature = "error-print")]
        eprintln!("Error: missing command line argument: the name of transactions file.");
        process::exit(1);
    }
    let filename = &args[1];
    match File::open(filename) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let mut accounts =
                AccountHub::new(|_client_id| Account::new(Box::new(InMemoryLedger::new())));
            accounts.process_csv(reader);
            if let Err(_err) = accounts.write_summary(&mut std::io::stdout()) {
                #[cfg(feature = "error-print")]
                eprint!("Error: {_err}\n");
                process::exit(3);
            }
        }
        Err(_err) => {
            #[cfg(feature = "error-print")]
            eprint!("Error: {_err} \"{filename}\"\n");
            process::exit(2);
        }
    };
}
