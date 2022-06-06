pub mod account;
pub mod account_hub;
pub mod amount;
pub mod in_memory_ledger;
pub mod ledger;

use pest::Parser;
use std::str::FromStr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

pub use crate::account_hub::*;
use crate::amount::Amount;

#[macro_use]
extern crate pest_derive;

#[derive(Parser)]
#[grammar = "actions.pest"]
struct ActionParser;

/// tuns a csv record into executable actions
fn parse_csv_line(line: &str) -> Result<(ClientId, Action), ParseError> {
    if let Ok(items) = ActionParser::parse(Rule::line_input, &line) {
        //we get here only with valid number of items thanks to the parser!
        let mut cid = Option::<ClientId>::None;
        let mut tid = Option::<TransactionId>::None;
        let mut amount = Option::<Amount>::None;
        let mut typ: Rule = Rule::EOI;

        for item in items {
            match item.as_rule() {
                Rule::client_id => cid = ClientId::from_str(item.as_str()).ok(),
                Rule::transaction_id => tid = TransactionId::from_str(item.as_str()).ok(),
                Rule::amount => amount = Amount::from_str(item.as_str()).ok(),
                Rule::deposit => typ = Rule::deposit,
                Rule::withdrawal => typ = Rule::withdrawal,
                Rule::dispute => typ = Rule::dispute,
                Rule::resolve => typ = Rule::resolve,
                Rule::charge_back => typ = Rule::charge_back,
                _ => {}
            };
        }

        if let (Some(cid), Some(tid)) = (cid, tid) {
            match (typ, amount) {
                (Rule::deposit, Some(amount)) => {
                    Some(Action::Transact((tid, Transaction::Deposit(amount))))
                }
                (Rule::withdrawal, Some(amount)) => {
                    Some(Action::Transact((tid, Transaction::Withdrawal(amount))))
                }
                (Rule::dispute, _) => Some(Action::Dispute(tid)),
                (Rule::resolve, _) => Some(Action::Resolve(tid)),
                (Rule::charge_back, _) => Some(Action::ChargeBack(tid)),
                _ => None,
            }
            .map(|action| (cid, action))
            .ok_or(ParseError)
        } else {
            Err(ParseError)
        }
    } else {
        Err(ParseError)
    }
}

/// Processes the lines of a csv file from 'reader'.
/// The "type, client, tx, amount" header is skipped, just like any other lines with parse error.
/// Executes the transactions given in well formed lines, the writes out the summary of each client account in csv format with
/// "client,available,held,total,locked" header line to 'writer'.
/// If "error-print" feature is enabled, failures are logged on stderr.
pub async fn process_csv<R, W, L>(
    mut accounts: AccountHub<L>,
    reader: R,
    writer: &mut W,
) -> Result<(), std::io::Error>
where
    R: AsyncBufReadExt + Unpin,
    W: AsyncWriteExt + Unpin + Send,
    L: Ledger + 'static,
{
    // spawn a task for logging action responses:
    let (response_sender, mut response_receiver) =
        mpsc::channel::<(Result<(), TransactionError>, (ClientId, Action))>(64);
    tokio::spawn(async move {
        while let Some((_response, (_client_id, _action))) = response_receiver.recv().await {
            #[cfg(feature = "error-print")]
            match _response {
                Ok(()) => eprintln!("## Transaction successful: {_client_id} {:?}", _action),
                Err(err) => {
                    eprintln!("## Transaction refused: {err} - {_client_id} {:?}", _action)
                }
            }
        }
    });

    // read the file and process the lines
    // a part of the possible errors returned immediately
    // the rest is collected by the above spawned task.
    let mut lines = reader.lines();
    while let Ok(Some(line)) = lines.next_line().await {
        match parse_csv_line(&line) {
            Ok((client_id, action)) => {
                if let Err(_err) = accounts.execute(client_id, action, &response_sender).await {
                    #[cfg(feature = "error-print")]
                    eprint!(
                        "Transaction refused: {_err} (client: {client_id} {:?})\n",
                        action
                    );
                }
            }
            Err(_err) => {
                #[cfg(feature = "error-print")]
                eprint!("Record skipped due to \"{_err}\" in \"{line}\"\n");
            }
        }
    }

    writer
        .write(b"client,available,held,total,locked\n")
        .await?;

    //summarize all started transactions
    let accounts = accounts.summarize().await;

    //write out the report
    for (client_id, account) in accounts {
        let summary = format!(
            "{}, {}, {}, {}, {}\n",
            client_id,
            account.available(),
            account.held(),
            account.total(),
            account.is_locked()
        );

        if let Err(_err) = writer.write(summary.as_bytes()).await {
            #[cfg(feature = "error-print")]
            eprint!("Was unable to write out summary \"{summary}\" due to error: \"{_err}\"\n");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::in_memory_ledger::InMemoryLedger;

    const INPUT: &[u8] = br###"type,   client, tx, amount
deposit, 1, 1, 1.0,
deposit,1, 2, 2    
deposit, 1, 3, .30 

deposit, 2, 4, 4.000000000000000    
deposit, 2, 5, 5.       
deposit, 2, 6, +6.0     
deposit, 2, 7, 5.0      

dispute, 1, 3,          
dispute, 1, 2           

deposit, 1, 8, + 1.2,   
deposit, 1, 30, - 1.2,
deposit_, 1, 9, 1.2   
deposit, a1, 10, 1.2  
deposit, -1, 11, 1.2  
deposit, 1.1, 12, 1.2 
deposit, 1, _13, 1.2  
deposit, 1, -14, 1.2  
deposit, 1, 15.2, 1.2 
deposit, 1, 16, _1.2   
deposit, 1, 17, 1. 2   
deposit, 1, 18, 1 .2   
deposit, 1, 19, 1.2e3, 
deposit, 1, 120, 1.00001,  
deposit, 1, 121, -1.00001,

deposit, 65536, 20, 1.2,
deposit, 1, 4294967296, 1.2
deposit, 1, 23, -1.2  
deposit, 1, 24, 922337203685477.5808  

, 1, 25, 1.2,
deposit, , 26, 1.2,
deposit, 1, , 1.2,
deposit, 1, 28, 
withdrawal, 1, 29, 
dispute, , 7
dispute, 1, 
resolve, 1,
resolve, , 7, 
chargeback, , 88
chargeback, 1, 

deposit, 10, 51, 1234567890.1234,    
deposit, 10, 42, 1.2,    
deposit, 10, 33, 0,    
dispute, 10, 45                         
withdrawal, 10, 55, 1234567890.3234,    
deposit, 10, 56, 922337203685476.5807,  
deposit, 10, 57, 0.0001,  

withdrawal, 50, 61, 0    
withdrawal, 50, 62, 1    
deposit, 50, 63, 100     
withdrawal, 50, 64, 0    
withdrawal, 50, 65, 5    
withdrawal, 50, 66, 99   
deposit, 50, 67, 200.124 
deposit, 50, 68, 1.00000 
resolve, 50, 63,         
chargeback, 50, 63,      
resolve, 50, 3,          
chargeback, 50, 2,       
dispute, 50, 62         
dispute, 50, 65          
deposit, 50, 67, 200     
dispute, 50, 63          
dispute, 50, 66          
dispute, 50, 63,         
resolve, 50, 63,         
chargeback, 50, 63,      
resolve, 50, 63,         
dispute, 50, 63,         
chargeback, 50, 63,      
chargeback, 50, 63,      
deposit, 50, 71, 200,    
withdrawal, 50, 72, 1,   
chargeback 50, 67        

dispute, 1, 3,           
withdrawal, 1, 80, 1.1   
withdrawal, 1, 80, 0.8   
chargeback, 1, 3         
chargeback, 1, 2         
dispute, 1, 1            
chargeback, 1, 1         

dispute, 2, 5,
"###;

    const OUTPUT: &[u8] = br###"client,available,held,total,locked
1, -0.8, 0, -0.8, true
2, 15, 5, 20, false
10, 922337203685477.5807, 0, 922337203685477.5807, false
50, 196.124, 0, 196.124, true
"###;

    #[tokio::test]
    async fn full_integration_test() {
        let mut summary_buff = Vec::<u8>::new();
        assert_eq!(
            process_csv(
                AccountHub::new(|_client_id| InMemoryLedger::connect()),
                INPUT,
                &mut summary_buff
            )
            .await
            .is_ok(),
            true
        );
        assert_eq!(summary_buff, OUTPUT);
    }
}
