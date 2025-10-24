# rust-payments-engine

## System Design Notes

- Only store deposits that might later be disputed; Withdrawals and other transactions are processed and discarded right away.
- Each client maintains its own map of transactions. This avoids global locks, keeps things cache-friendly, and scales better when there are many clients. (A single global map would use less memory, but it makes concurrency messier.)
- Transaction types are defined as enum so the compiler enforces business rules instead of relying on string comparisons at runtime.
- The `process_transactions` function works on streams, wrapped with BufReader/BufWriter. This lets it handle huge CSVs or even incoming data from multiple TCP streams without loading everything into memory.
- If a CSV line fails to parse, it just logs it and moves on. For bigger structural issues (like missing amounts), it throws a specific MissingAmount error so it’s obvious what went wrong.
- I’ve tried to keep the code clean and readable first, rather than chasing micro-optimisations too early.
- There are unit tests for all the main state transitions and helpers.
