use crate::actions::TransactionId;
use crate::amount::Amount;
use std::collections::HashMap;

/// abstraction over a key-value pair storage
pub trait Ledger {
    type Error;
    type Key;
    type Value;

    /// returns true if the given key is already in the storage (or error)
    fn contains(&self, key: Self::Key) -> Result<bool, Self::Error> {
        self.get(key).map(|_| true)
    }

    /// returns value for given key is already in the storage (or error)
    fn get(&self, key: Self::Key) -> Result<Option<Self::Value>, Self::Error>;

    /// inserts/updates the value in the storage belongs to the given key (or error)
    /// must always check if returned with success! (a real db could return Err<DbError>)
    /// NOTE: if the network would lose the response of the server that is a big problem!!!
    #[must_use]
    fn insert(&mut self, key: Self::Key, state: Self::Value) -> Result<(), Self::Error>;
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

/// An in-memory implementation of 'Ledger'
/// Hopefully this fits in memory (in worst case 64GB memory usage estimated),
/// but persistent storage would be better (or required if the message history is not archived elsewhere)
/// (Vec would use somewhat less memory, but slower, allocated in one large block)
pub struct InMemoryLedger {
    db: HashMap<TransactionId, TransactionState>,
}

impl InMemoryLedger {
    pub fn new() -> Self {
        Self {
            db: HashMap::<TransactionId, TransactionState>::new(),
        }
    }
}

impl Ledger for InMemoryLedger {
    type Error = ();
    type Key = TransactionId;
    type Value = TransactionState;

    fn contains(&self, key: Self::Key) -> Result<bool, Self::Error> {
        //real db could return Err<DbError>
        Ok(self.db.contains_key(&key))
    }

    fn get(&self, key: Self::Key) -> Result<Option<TransactionState>, Self::Error> {
        //real db could return Err<DbError>
        Ok(self.db.get(&key).map(|v| *v))
    }

    /// must always check if returned with success!
    /// (a real db could return Err<DbError>)
    #[must_use]
    fn insert(&mut self, key: Self::Key, state: TransactionState) -> Result<(), Self::Error> {
        self.db.insert(key, state);
        Ok(())
    }
}
