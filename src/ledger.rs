use async_trait::async_trait;
use std::fmt::Display;
use std::str::FromStr;

pub use crate::amount::*;

/// Transaction ids wrapped in new type to avoid mixing them with other ids
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub struct TransactionId(u32);

impl From<u32> for TransactionId {
    fn from(v: u32) -> Self {
        TransactionId(v)
    }
}

impl Display for TransactionId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for TransactionId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        u32::from_str(s).map(|id| TransactionId(id))
    }
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

//transaction ledger trait
#[async_trait]
pub trait Ledger: Send + Sync {
    type Error: Send + Sync;

    /// returns true if the given key is already in the storage (or error)
    async fn contains(&self, key: TransactionId) -> Result<bool, Self::Error>;

    /// returns value for given key is already in the storage (or error)
    async fn get(&self, key: TransactionId) -> Result<Option<TransactionState>, Self::Error>;

    /// inserts/updates the value in the storage belongs to the given key (or error)
    /// must always check if returned with success! (a real db could return Err<DbError>)
    /// NOTE: if the network would lose the response of the server that is a big problem!!!
    #[must_use]
    async fn insert(
        &mut self,
        key: TransactionId,
        state: TransactionState,
    ) -> Result<(), Self::Error>;
}
