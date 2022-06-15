use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg};
use log::error;
use std::env;
use std::process;
use tokio::fs::File;

use accounter::in_memory_ledger::*;
use accounter::*;

fn main() {
    dotenv::dotenv().ok(); //looks fo .env file to set up environment variables, command arguments
    pretty_env_logger::init();

    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::new("transactions file name")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let filename = match matches.get_one::<String>("transactions file name") {
        Some(filename) => filename,
        None => {
            //unexpected, we should never get here (clap exits if required argument is missing)
            error!("missing argument.");
            process::exit(3);
        }
    };
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        match File::open(filename).await {
            Ok(file) => {
                let capacity = 0x1000;
                let reader = tokio::io::BufReader::with_capacity(capacity, file);
                let mut writer = tokio::io::stdout();
                if let Err(_err) = process_csv(
                    AccountHub::new(|_client_id| InMemoryLedger::connect()),
                    reader,
                    &mut writer,
                )
                .await
                {
                    error!("{_err}");
                    process::exit(5);
                }
            }
            Err(_err) => {
                error!("{_err} \"{filename}\"");
                process::exit(4);
            }
        };
    });
}
