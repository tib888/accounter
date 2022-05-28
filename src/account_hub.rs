/// This hub has two main purpose:
/// * it is the owner of all Accounts, does lifetime management
/// * it is responsible to forward requests to the right Account actor
use crate::account::*;
use crate::actions::Action;
use crate::actions::*;
use crate::amount::{Amount, ParseError};
use crate::ledger::*;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{self, Sender};
use tokio::task::JoinHandle;

use std::cmp::{Ord, Ordering};
use std::collections::BTreeMap;
use std::fmt::Display;

use std::str::FromStr;

use pest::Parser;

/// used to address the Accounts managed accounts
#[derive(Debug, PartialEq, Clone, Copy, Eq, PartialOrd)]
pub struct ClientId(u16);

impl From<u16> for ClientId {
    fn from(v: u16) -> Self {
        ClientId(v)
    }
}

impl Ord for ClientId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ClientId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        u16::from_str(s).map(|id| ClientId(id))
    }
}

/// owner of client accounts, entry point to access them
pub struct AccountHub {
    accounts: BTreeMap<ClientId, (Sender<Action>, JoinHandle<(ClientId, Account)>)>,
}

impl AccountHub {
    /// When a 'fresh' ClientId received by AccountHub, it needs to create a
    /// new account - for that the give lambda function is used
    /// this way easy to switch lambda ledger implementations
    pub fn new() -> Self {
        AccountHub {
            accounts: BTreeMap::<ClientId, (Sender<Action>, JoinHandle<(ClientId, Account)>)>::new(
            ),
        }
    }

    /// forward the given action request message to the account addressed by client_id
    /// if it not exist yet a new account is created automatically by the lambda function
    /// passed to the AccountHub::new
    async fn execute(
        &mut self,
        client_id: ClientId,
        action: Action,
        response_sender: &Sender<(Result<(), TransactionError>, (ClientId, Action))>,
    ) -> Result<(), SendError<Action>> {
        if let Some((action_sender, _join_handle)) = self.accounts.get(&client_id) {
            //if the client is already known, simply send the action for processing by his account
            action_sender.send(action).await
        } else {
            //for new clients an account with a transaction database has to be created
            //and on success send the first action for processing by his account
            match InMemoryLedger::connect() {
                Ok(ledger) => {
                    let (action_sender, mut action_receiver) = mpsc::channel::<Action>(16);
                    let mut account = Account::new(Box::new(ledger));
                    let responder = response_sender.clone(); //each spawned task has his own sender to the response channel

                    // for each account spawn a task which processes his actions form the channel
                    let join_handle: JoinHandle<_> = tokio::spawn(async move {
                        #[cfg(feature = "trace-print")]
                        eprintln!("> {client_id} spawned");

                        while let Some(action) = action_receiver.recv().await {
                            #[cfg(feature = "trace-print")]
                            eprintln!("> {client_id} executing: {:?}", action);

                            let response = account.execute(action).await;

                            //if "error-print" feature is not enable will execute faster (not sending responses, no queue syncing is needed)
                            #[cfg(feature = "error-print")]
                            let _err = responder.send((response, (client_id, action))).await;
                            //discard possible error
                        }

                        #[cfg(feature = "trace-print")]
                        eprintln!("> {client_id} is stopped.");
                        (client_id, account)
                    });
                    let result = action_sender.send(action).await; //send the first action!
                    self.accounts
                        .insert(client_id, (action_sender, join_handle));
                    result
                }
                Err(_) => {
                    #[cfg(feature = "error-print")]
                    eprint!("Transaction refused: Database connection failed (client: {client_id} {:?})\n", action);
                    Ok(())
                }
            }
        }
    }

    /// processes the lines of a csv file
    /// "type, client, tx, amount" header is skipped
    /// just like any other lines with parse error
    /// executes the transactions given in well formed lines
    ///
    /// writes out the account summary of each client in csv format with
    /// "client,available,held,total,locked" header line
    ///
    /// if "error-print" feature is enabled, failures are logged on stderr
    pub async fn process_csv<R, W>(
        &mut self,
        reader: R,
        writer: &mut W,
    ) -> Result<(), std::io::Error>
    where
        R: AsyncBufReadExt + Unpin,
        W: AsyncWriteExt + Unpin + Send,
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
                    if let Err(_err) = self.execute(client_id, action, &response_sender).await {
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

        //drop the sender of every account -> they will exit from their spawned task and returning summary
        writer
            .write(b"client,available,held,total,locked\n")
            .await?;

        //TODO Nightly has "pop_first"
        //luckily the BTreeMap is sorted by key, so always produces the same result (good for unit tests).
        let clients: Vec<_> = self.accounts.keys().cloned().collect();
        for client in clients {
            if let Some((sender, join_handle)) = self.accounts.remove(&client) {
                drop(sender);
                if let Ok((client_id, account)) = join_handle.await {
                    #[cfg(feature = "trace-print")]
                    eprint!("> closed {client_id}\n");

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
            }
        }

        Ok(())
    }
}

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
                (Rule::deposit, Some(amount)) => Some(Action::Transact(TransactionData {
                    id: tid,
                    transaction: Transaction::Deposit(amount),
                })),
                (Rule::withdrawal, Some(amount)) => Some(Action::Transact(TransactionData {
                    id: tid,
                    transaction: Transaction::Withdrawal(amount),
                })),
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut accounts = AccountHub::new();
        assert_eq!(
            accounts.process_csv(INPUT, &mut summary_buff).await.is_ok(),
            true
        );
        assert_eq!(summary_buff, OUTPUT);
    }
}
