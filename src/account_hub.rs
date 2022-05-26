/// This hub has two main purpose:
/// * it is the owner of all Accounts, does lifetime management
/// * it is responsible to forward requests to the right Account actor
use crate::account::*;
use crate::actions::Action;
use crate::actions::*;
use crate::amount::{Amount, ParseError};
use std::io::{BufRead, Write};

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
    pub accounts: BTreeMap<ClientId, Account>,
    account_creator: fn(ClientId) -> Account,
}

impl AccountHub {
    /// When a 'fresh' ClientId received by AccountHub, it needs to create a
    /// new account - for that the give lambda function is used
    /// this way easy to switch lambda ledger implementations
    pub fn new(account_creator: fn(ClientId) -> Account) -> Self {
        AccountHub {
            accounts: BTreeMap::<ClientId, Account>::new(),
            account_creator: account_creator,
        }
    }

    /// forward the given action request message to the account addressed by client_id
    /// if it not exist yet a new account is created automatically by the lambda function
    /// passed to the AccountHub::new
    pub fn execute(&mut self, client_id: ClientId, action: Action) -> Result<(), TransactionError> {
        self.accounts
            .entry(client_id)
            .or_insert((self.account_creator)(client_id))
            .execute(action)
    }

    /// processes the lines of a csv file
    /// "type, client, tx, amount" header is skipped
    /// just like any other lines with parse error
    /// executes the transactions given in well formed lines
    /// if "error-print" feature is enabled, failures are logged on stderr
    pub fn process_csv(&mut self, reader: impl BufRead) {
        for line in reader.lines() {
            if let Ok(line) = &line {
                match parse_csv_line(&line) {
                    Ok((client_id, action)) => {
                        if let Err(_err) = self.execute(client_id, action) {
                            #[cfg(feature = "error-print")]
                            eprint!("Transaction refused: \"{line}\" -- Error: {}\n", _err);
                        }
                    }

                    Err(_err) => {
                        #[cfg(feature = "error-print")]
                        eprint!("Record skipped due to \"{_err}\" in \"{line}\"\n");
                    }
                }
            }
        }
    }

    /// writes out the account summary of each client in csv format with
    /// "client,available,held,total,locked" header line
    pub fn write_summary(&self, writer: &mut impl Write) -> Result<(), std::io::Error> {
        write!(writer, "client,available,held,total,locked\n").and_then(|()| {
            for item in &self.accounts {
                let client = item.0;
                let account = item.1;
                write!(
                    writer,
                    "{}, {}, {}, {}, {}\n",
                    client,
                    account.available(),
                    account.held(),
                    account.total(),
                    account.is_locked()
                )?;
            }
            Ok(())
        })
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
    use crate::ledger::*;

    const INPUT: &[u8] = br###"type,   client, tx, amount
deposit, 1, 1, 1.0,
deposit,1, 2, 2    
deposit, 1, 3, .30 

deposit, 2, 4, 4.000000000000001    
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
deposit, 50, 68, 1.00001 
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

    #[test]
    fn full_integration_test() {
        let mut accounts =
            AccountHub::new(|_client_id| Account::new(Box::new(InMemoryLedger::new())));
        accounts.process_csv(INPUT);
        let mut buff = Vec::<u8>::new();
        let _err = accounts.write_summary(&mut buff);
        assert_eq!(buff, OUTPUT);
    }
}
