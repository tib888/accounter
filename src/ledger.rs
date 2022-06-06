use crate::actions::TransactionId;
use crate::amount::Amount;

use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fmt::Display;

#[cfg(feature = "simulate-delays")]
use tokio::time::{sleep, Duration};

/// abstraction over a key-value pair storage
#[async_trait]
pub trait Ledger: Send + Sync {
    type Error: Send + Sync;
    type Key;
    type Value;

    /// returns true if the given key is already in the storage (or error)
    async fn contains(&self, key: Self::Key) -> Result<bool, Self::Error>;

    /// returns value for given key is already in the storage (or error)
    async fn get(&self, key: Self::Key) -> Result<Option<Self::Value>, Self::Error>;

    /// inserts/updates the value in the storage belongs to the given key (or error)
    /// must always check if returned with success! (a real db could return Err<DbError>)
    /// NOTE: if the network would lose the response of the server that is a big problem!!!
    #[must_use]
    async fn insert(&mut self, key: Self::Key, state: Self::Value) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransactionState {
    Deposit(Amount),
    DepositInDispute(Amount),
    ChargedBack(Amount),
    //InDisputeWithdrawal(Amount),  //TODO ASK! - I assumed that there is no such thing as withdrawal dispute.
    Withdrawal(Amount), //TODO ASK! this could be omitted theoretically if Withdrawal disputes are not possible,
                        //          but in that case state restore from persisted ledger database (by transaction replay)
                        //          would not be possible, so I leave this here...
}

#[derive(Debug, PartialEq, Eq)]
pub struct LedgerError;

impl Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ledger error")
    }
}

impl Error for LedgerError {}

/// An in-memory implementation of 'Ledger'
/// Hopefully this fits in memory (in worst case 64GB memory usage estimated),
/// but persistent storage would be better (or required if the message history is not archived elsewhere)
/// (Vec would use somewhat less memory, but slower, allocated in one large block)
#[derive(Debug)]
pub struct InMemoryLedger {
    db: HashMap<TransactionId, TransactionState>,
}

impl InMemoryLedger {
    /// simulate a db connection
    pub fn connect() -> Option<Self> {
        Some(Self {
            db: HashMap::<TransactionId, TransactionState>::new(),
        })
    }
}

#[async_trait]
impl Ledger for InMemoryLedger {
    type Error = LedgerError;
    type Key = TransactionId;
    type Value = TransactionState;

    async fn contains(&self, key: Self::Key) -> Result<bool, Self::Error> {
        #[cfg(feature = "simulate-delays")]
        sleep(Duration::from_millis(1000)).await;

        //real db could return Err<DbError>
        Ok(self.db.contains_key(&key))
    }

    async fn get(&self, key: Self::Key) -> Result<Option<TransactionState>, Self::Error> {
        #[cfg(feature = "simulate-delays")]
        sleep(Duration::from_millis(1000)).await;

        //real db could return Err<DbError>
        Ok(self.db.get(&key).map(|v| *v))
    }

    /// must always check if returned with success!
    /// (a real db could return Err<DbError>)
    #[must_use]
    async fn insert(&mut self, key: Self::Key, state: TransactionState) -> Result<(), Self::Error> {
        #[cfg(feature = "simulate-delays")]
        sleep(Duration::from_millis(1000)).await;

        self.db.insert(key, state);
        Ok(())
    }
}
