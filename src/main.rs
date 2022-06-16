use clap::Parser;
use log::error;
use std::process;
use tokio::fs::File;

use accounter::in_memory_ledger::*;
use accounter::*;

#[derive(Parser, Debug)]
#[clap(author, about, version)]
struct Args {
    /// Transactions file name
    #[clap()]
    filename: String,

    /// Log level filters
    /// [possible values: Off, Error, Warn, Info, Debug, Trace]
    #[clap(short('l'), long, env("ACCOUNTS_LOG_LEVEL"))]
    log_level: Option<String>,

    /// Log write style
    /// [possible values: Auto | Never | Always]
    #[clap(short('s'), long, env("ACCOUNTS_LOG_STYLE"))]
    log_style: Option<String>,
}

fn main() {
    dotenv::dotenv().ok(); //looks for .env file in the current and parent folders to set up environment variables
    let args = Args::parse(); //reads command arguments (which may come from environment variables too)

    pretty_env_logger::formatted_builder()
        .parse_filters(&args.log_level.unwrap_or(String::default()))
        .parse_write_style(&args.log_style.unwrap_or(String::default()))
        .init();

    tokio::runtime::Runtime::new().unwrap().block_on(async {
        match File::open(&args.filename).await {
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
                error!("{_err} \"{}\"", &args.filename);
                process::exit(4);
            }
        };
    });
}
