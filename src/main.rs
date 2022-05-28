pub mod account;
pub mod account_hub;
pub mod actions;
pub mod amount;
pub mod ledger;

use crate::account_hub::*;

use std::env;
use std::process;

use tokio::fs::File;

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

    tokio::runtime::Runtime::new().unwrap().block_on(async {
        match File::open(filename).await {
            Ok(file) => {
                let mut accounts = AccountHub::new();
                let capacity = 0x1000;
                let reader = tokio::io::BufReader::with_capacity(capacity, file);
                let mut writer = tokio::io::stdout();
                if let Err(_err) = accounts.process_csv(reader, &mut writer).await {
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
    });
}
