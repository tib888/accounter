use log::error;
use std::env;
use std::process;
use tokio::fs::File;

use accounter::in_memory_ledger::*;
use accounter::*;

fn main() {
    pretty_env_logger::init();
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        error!("Error: missing command line argument: the name of transactions file.");
        process::exit(1);
    }
    let filename = &args[1];

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
                    error!("Error: {_err}");
                    process::exit(3);
                }
            }
            Err(_err) => {
                error!("Error: {_err} \"{filename}\"");
                process::exit(2);
            }
        };
    });
}
