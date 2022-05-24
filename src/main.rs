pub mod actions;
pub mod amount;

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process;
use std::str::FromStr;

use pest::Parser;

use crate::actions::*;
use crate::amount::Amount;

#[macro_use]
extern crate pest_derive;
#[derive(Parser)]
#[grammar = "actions.pest"]
struct ActionParser;

#[derive(Debug, PartialEq, Clone, Copy, PartialOrd)]
pub struct ClientId(u16);

impl From<u16> for ClientId {
    fn from(v: u16) -> Self {
        ClientId(v)
    }
}

pub fn parse_from_csv(line: &str) -> Option<(ClientId, Action)> {
    if let Ok(items) = ActionParser::parse(Rule::line_input, &line) {
        //we get here only with valid number of items thanks to the parser!
        let mut cid = 0u16;
        let mut tid = 0u32;
        let mut amount = Option::<&str>::None;
        let mut typ: Rule = Rule::EOI;

        for item in items {
            match item.as_rule() {
                Rule::client_id => {
                    if let Ok(v) = item.as_str().parse() {
                        cid = v;
                    }
                }
                Rule::transaction_id => {
                    if let Ok(v) = item.as_str().parse() {
                        tid = v;
                    }
                }
                Rule::amount => {
                    amount = Some(item.as_str());
                }
                Rule::deposit => typ = Rule::deposit,
                Rule::withdrawal => typ = Rule::withdrawal,
                Rule::dispute => typ = Rule::dispute,
                Rule::resolve => typ = Rule::resolve,
                Rule::chargeback => typ = Rule::chargeback,
                _ => {}
            };
        }

        let action = match typ {
            Rule::deposit => {
                amount
                    .and_then(|amount| Amount::from_str(amount).ok())
                    .map(|amount| {
                        Action::Transact(TransactionData {
                            id: TransactionId::from(tid),
                            transaction: Transaction::Deposit(amount),
                        })
                    })
            }
            Rule::withdrawal => {
                amount
                    .and_then(|amount| Amount::from_str(amount).ok())
                    .map(|amount| {
                        Action::Transact(TransactionData {
                            id: TransactionId::from(tid),
                            transaction: Transaction::Withdrawal(amount),
                        })
                    })
            }
            Rule::dispute => Some(Action::Dispute(TransactionId::from(tid))),
            Rule::resolve => Some(Action::Resolve(TransactionId::from(tid))),
            Rule::chargeback => Some(Action::Chargeback(TransactionId::from(tid))),
            _ => None,
        };

        action.map(|action| (ClientId::from(cid), action))
    } else {
        None
    }
}

fn process<T>(reader: T)
where
    T: BufRead,
{
    for line in reader.lines() {
        if let Ok(line) = &line {
            let action = parse_from_csv(&line);
            println!("{}\t-\t{:?}", &line, &action);
        }
    }
}

fn start(filename: &str) {
    if let Ok(file) = File::open(filename) {
        let reader = BufReader::new(file);
        process(reader);
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

#[cfg(test)]
mod tests {
    #[test]
    fn simple() {
        let text = "type, client, tx, amount\ndeposit, 1, 1, 1.0\ndeposit, 2, 2, 2.0\ndeposit, 1, 3, 2.0\nwithdrawal, 1, 4, 1.5\nwithdrawal, 2, 5, 3.0\n";
        super::process(text.as_bytes());
    }
}
