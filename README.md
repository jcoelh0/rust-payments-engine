# rust-payments-engine

## System Design Notes

- Only store deposits that might later be disputed; Withdrawals and other transactions are processed and discarded right away.
- Each client maintains its own map of transactions. This avoids global locks, keeps things cache-friendly, and scales better when there are many clients. (A single global map would use less memory, but it makes concurrency messier.)
- Transaction types are defined as enum so the compiler enforces business rules instead of relying on string comparisons at runtime.
- The `process_transactions` function works on streams, wrapped with BufReader/BufWriter. This lets it handle huge CSVs or even incoming data from multiple TCP streams without loading everything into memory.
- Error handling (`EngineError` and `ClientTransactionError`) covers client operations misuse, io/csv parsing, account errors, and validation failures such as missing amounts or non-positive ids/amounts.
- A configurable read buffer could batch multiple CSV rows per socket read when embedding the engine behind TCP streams, making it faster under heavy traffic.
- There are unit tests for all the transaction states and helpers.
- Since the field `total` is `available + held`, we could remove `total` and just return the sum them.
- Another solution to accomodate the requirement of 4 decimal precision, instead of using the crate Decimal, would be to use Integers where 1 would be equivalent 0.0001 (multiplying values by 10000).
- Transactions with non-positive transaction IDs or amounts are validated, logged, and skipped so the stream keeps going without crashing.
------------

## AI Usage Disclosure

I used AI to assist with error handling structure (thiserror crate) and test scaffolding.
The overall structure, design, and decision to implement custom error types with the thiserror crate were entirely my own.
The prompts helped with:
	• Error handling: generating boilerplate and refining the syntax for two custom error types under src/errors, based on my input of the failure points.
	• Testing: generating initial test cases after I described the edge cases, inputs, and expected outputs. I reviewed, adjusted, and expanded all tests manually to ensure full coverage and correctness.

All architecture, logic, and implementation choices were mine. The AI support was limited to code refinement and syntax assistance.
