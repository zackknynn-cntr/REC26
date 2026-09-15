# REC26 Rust Server

Compatibility/preservation backend targeting Rec Room PC build **20231207**.

## Run
```powershell
cargo run
```
The dashboard is the terminal itself. No browser UI is required.

Keys: `Q`/`Esc` quits, `C` clears the visible log history.

The server binds to `0.0.0.0:9000`; clients on the same machine use `127.0.0.1:9000`.

## Important
Several response bodies are compatibility probes reconstructed from observed client behavior. They are intentionally not claimed to be exact historical Rec Room schemas. This project does not disable or bypass anti-cheat, integrity verification, signatures, or certificate pinning.
