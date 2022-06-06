/// This hub has two main purpose:
/// * it is the owner of all Accounts, does lifetime management
/// * it is responsible to forward requests to the right Account actor
use std::cmp::Ord;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::str::FromStr;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{self, Sender};
use tokio::task::JoinHandle;

pub use crate::account::*;

/// Client ids wrapped in new type to avoid mixing them with other ids.
/// Used to address the accounts managed by AccountHub.
#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
pub struct ClientId(u16);

impl From<u16> for ClientId {
    fn from(v: u16) -> Self {
        ClientId(v)
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

/// Owner of client accounts, entry point to access them.
#[derive(Debug)]
pub struct AccountHub<L> {
    accounts: BTreeMap<ClientId, (Sender<Action>, JoinHandle<(ClientId, Account<L>)>)>,
    ledger_connector: fn(ClientId) -> Option<L>,
}

impl<L> AccountHub<L>
where
    L: Ledger + 'static,
{
    /// When a 'fresh' ClientId received by AccountHub, it creates a new account using
    /// the given 'ledger_connector' lambda function.
    /// This way easy to switch ledger implementations.
    pub fn new(ledger_connector: fn(ClientId) -> Option<L>) -> Self {
        AccountHub {
            accounts:
                BTreeMap::<ClientId, (Sender<Action>, JoinHandle<(ClientId, Account<L>)>)>::new(),
            ledger_connector: ledger_connector,
        }
    }

    /// Forwards the given action request message to the account addressed by client_id.
    /// If it not exists yet, a new account is created automatically by the lambda function
    /// passed to the AccountHub::new
    pub async fn execute(
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
            match (self.ledger_connector)(client_id) {
                Some(ledger) => {
                    let (action_sender, mut action_receiver) = mpsc::channel::<Action>(16);
                    let mut account = Account::new(ledger);
                    let responder = response_sender.clone(); //each spawned task has his own sender to the response channel

                    // for each account spawn a task which processes his actions form the channel
                    let join_handle: JoinHandle<_> = tokio::spawn(async move {
                        while let Some(action) = action_receiver.recv().await {
                            let response = account.execute(action).await;

                            //if "error-print" feature is not enable will execute faster (not sending responses, no queue syncing is needed)
                            #[cfg(feature = "error-print")]
                            let _err = responder.send((response, (client_id, action))).await;
                            //discard possible error
                        }

                        (client_id, account)
                    });
                    let result = action_sender.send(action).await; //send the first action!
                    self.accounts
                        .insert(client_id, (action_sender, join_handle));
                    result
                }
                _ => {
                    #[cfg(feature = "error-print")]
                    eprint!("Transaction refused: Database connection failed (client: {client_id} {:?})\n", action);
                    Ok(())
                }
            }
        }
    }

    /// Returns the state of accounts after all actions executed.
    /// Consumes self - this way blocks sending further actions for execution.
    pub async fn summarize(mut self) -> Vec<(ClientId, Account<L>)> {
        let mut accounts = Vec::<(ClientId, Account<L>)>::new();
        //TODO Nightly has "pop_first"
        //luckily the BTreeMap is sorted by key, so always produces the same result (good for unit tests).
        let clients: Vec<_> = self.accounts.keys().cloned().collect();
        for client in clients {
            if let Some((sender, join_handle)) = self.accounts.remove(&client) {
                //drop the sender of every account -> they will exit from their spawned task and returning summary
                drop(sender);
                if let Ok(account) = join_handle.await {
                    accounts.push(account);
                }
            }
        }
        accounts
    }
}
