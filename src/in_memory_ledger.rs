use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fmt::Display;
#[cfg(feature = "simulate-delays")]
use tokio::time::{sleep, Duration};

use crate::ledger::*;

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

    async fn contains(&self, key: TransactionId) -> Result<bool, Self::Error> {
        #[cfg(feature = "simulate-delays")]
        sleep(Duration::from_millis(1000)).await;

        //real db could return Err<DbError>
        Ok(self.db.contains_key(&key))
    }

    async fn get(&self, key: TransactionId) -> Result<Option<TransactionState>, Self::Error> {
        #[cfg(feature = "simulate-delays")]
        sleep(Duration::from_millis(1000)).await;

        //real db could return Err<DbError>
        Ok(self.db.get(&key).copied())
    }

    /// must always check if returned with success!
    /// (a real db could return Err<DbError>)
    #[must_use]
    async fn insert(
        &mut self,
        key: TransactionId,
        state: TransactionState,
    ) -> Result<(), Self::Error> {
        #[cfg(feature = "simulate-delays")]
        sleep(Duration::from_millis(1000)).await;

        self.db.insert(key, state);
        Ok(())
    }
}
