
I collected my thoughts during development in a 'blog' form below.

(see also the "//TODO ASK!" comments in the code):

* About testing:  test_data/In test_explained.csv I tried to collect all possible problems the input file may have. test_data/transactions.csv is created from that without the comments, the expected output is in test_data/expected.csv. An end to end test is also created base on this: account_hub.rs / full_integration_test

* Unit test will be added to every important public API of the module 

* ASSUMPTION: No need to check file header.

* ASSUMPTION: Wrong records can be skipped without exiting with error.

* ASSUMPTION: More items in a record than need is fine - those are skipped.

* QUESTION: Is it possible to dispute a withdrawal transaction? - ASSUMED: Is it NOT possible.

* ASSUMPTION: (2^63-1)/10000.0 unit per transaction is enough (922_337_203_685_477.5807)

* ASSUMPTION: (2^63-1)/10000.0 unit is enough for user balance calculations too

* ASSUMPTION: Zero or negative deposit / withdrawal amount is illegal.

* ASSUMPTION: if more than 4 fraction digits used for amount, and those are non zero, then that is illegal amount -> error.

* IDEA: It would be a good idea to limit the max amount per transaction - add sanity check based on that to protect against erroneous data.

* ASSUMPTION: disputes can be started even on locked accounts

* QUESTION: Should we allow deposits onto locked accounts? - ASSUMED: No.

* IDEA: decorate Amount and Account with a marker type - this way would be impossible to mix different units (USD/EUR/etc.)

* NOTE: Pest parser chosen to parse and validate csv file content -> fast, easy to modify the syntax in "actions.pest"

* ASSUMPTION: For this test I assumed that the server will contain enough memory to keep the transaction log in memory even in the worst case ( 2^32 transaction )
Better would be: store the ledger in an external database and have persistency.

* NOTE: The paper saids that the account has to be frozen on charge backs but gives no option to return to normal state.
So withdraw of remaining funds (if there is any) would be impossible forever.
Also deposit to cover the possible loss (and potentially automatically unlock) is also not possible.

* IDEAS:  An 'unlock' function is likely needed (at least when the available amount never went to negative). Also one may want to enumerate open disputes / executed charge backs (these states are stored in the ledger). -- These and current balance query could be implemented in further actions (with receiver parameters)...

* //NOTE: "error-print" feature introduced - may be removed form default features if no output required on stderr

* NOTE: To simulate the real-world requirements, where the transaction messages are likely coming as network messages, and the ledger database is likely connected also trough network, I'll switch to async using tokio runtime.

* NOTE: Performance comparison could be done later...

* NOTE: If we assume that a transactions processing can be slow, we can speed processing up if we spawn their execution:
Each client account could have its own message queue in which the order of his transactions is be kept

* The spawned execution is ready, improved the artificially slowed down test case speed from 77s to 34s.

* Since transaction processing is async, error reporting is not immediate - a response collector task is spawned, which collects the responses from accounts
In this test project this is not needed, but in real life likely it would be...

* //NOTE: turning off "error-print" feature will improve processing speed (not sending responses, no queue syncing is needed, etc.)

* NOTE: It would be nice to add some real stress tests for speed and memory usage.
However the estimation is that we use about 16 bytes per transaction so the server should have more than 64Gb memory (or the InMemoryLedger have to be replaced...)

NOTE: "error-print" feature is replaced with pretty_env_logger - RUST_LOG environment variable sets the logging level (for example "trace")