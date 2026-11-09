# open-records

Pull US public records straight from the agencies into a local SQLite file on your own machine. No account, no API key, no broker in the middle, nobody sees what you look up.

Status: first cut (v0.1). Two connectors work today. The rest of the plan is in the roadmap below.

## Use

```
cargo run --release -- edgar AAPL          # recent SEC filings for a company (ticker, name or CIK)
cargo run --release -- awards Lockheed Martin   # federal contract awards by recipient (USAspending)
cargo run --release -- show                # print stored filings
cargo run --release -- show awards         # print stored awards
```

Everything lands in `./open-records.sqlite` (set `OPEN_RECORDS_DB` to move it). It is a normal SQLite file: query it with `sqlite3`, Python, or any local LLM tool.

## Roadmap (what a microgrant would fund)

- County property roll connectors for NY, FL, TX and PA (each county differs, which is why nobody has done it)
- FCC license and ownership, USAC E-Rate, FAA aircraft registry connectors
- An MCP server over the local file so any local agent can query it offline
- Offline data bundles so it works with no network after the first pull
- One-command install: `cargo install` plus prebuilt binaries

## Why

The records are public and free. The tools to join them without handing your queries to a data broker are not. This puts that tooling in the user's hands.

MIT licensed. Maintainer: Matthew Busel, mattbusel@tensorust.com.
