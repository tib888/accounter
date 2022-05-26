Development 'blog' (see also the "//TODO ASK!" comments in the code):

In test_explained.csv I tried to collect all possible problems the input file may have.
transactions.csv is created from that without the comments.
A unit test is also created base on this: account_hub.rs / full_integration_test

0. ASSUMPTION: No need to check file header. 
   ASSUMPTION: Wrong records can be skipped without exiting with error.
   ASSUMPTION: More items in a record than need is fine - those are skipped. 

1. QUESTION: Is it possible to dispute a withdrawal transaction? 
   ASSUMED: Is it NOT possible to dispute a withdrawal transaction.

2. ASSUMPTION: (2^63-1)/10000.0 unit per transaction is enough (922_337_203_685_477.5807)
3. ASSUMPTION: (2^63-1)/10000.0 unit is enough for user balance calculations too

4. ASSUMPTION: disputes can be started even on locked accounts

5. QUESTION: Should we allow deposits onto locked accounts?
   ASSUMED: No.

6. IDEA: decorate Amount and Account with a marker type - this way would be impossible to mix different units (USD/EUR/etc.)

7. NOTE: Pest parser chosen to parse and validate csv file content -> fast, easy to modify the syntax in "actions.pest"

8. ASSUMPTION: For this test I assumed that the server will contain enough memory to keep the transaction log in memory even in the worst case ( 2^32 transaction )
   Better would be: store the ledger in an external database and have persistency.

8. ASSUMPTION: Zero or negative deposit / withdrawal amount is illegal.

9. NOTE: The paper saids that the account has to be frozen on charge backs but gives no option to return to normal state. 
         So withdraw of remaining funds (if there is any) would be impossible forever.
         Also deposit to cover the possible loss (and potentially automatically unlock) is also not possible.
         // An 'unlock' function is likely needed (at least when the available amount never went to negative)...
         // Also one may want to enumerate open disputes / executed charge backs (these states are stored in the ledger)...

10. NOTE: "error-print" feature introduced - may be removed form default features if no output required on stderr

11. To simulate the real-world requirements, where the transaction messages are likely coming as network messages, 
    and the ledger database is likely connected also trough network, I'll switch to async using tokio runtime.
    (Performance comparison could be done later...)