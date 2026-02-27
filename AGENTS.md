## Cursor Cloud specific instructions

### Rust Blockchain (primary project)

Fully functional blockchain with REST API built in Rust using axum.

**Build & run:** `cargo build && cargo run` (serves on `0.0.0.0:8080`)

**Tests:** `cargo test` (5 unit tests covering genesis, mining, validation, tamper detection, PoW)

**API endpoints:**
- `GET /chain` — returns full blockchain
- `POST /mine` — mines a new block (body: `{"data":"..."}`)
- `GET /validate` — validates chain integrity

### Legacy Python scripts

- `Blockchain.py` — original Python blockchain sketch (has known code bugs, not fully functional)
- `Caesar-Cipher/Cipher.py` — Caesar cipher, runs with `python3 Caesar-Cipher/Cipher.py`

### Notes

- No linter or CI pipeline is configured.
- The Rust project uses `Cargo.lock` for reproducible builds. The update script runs `pip install flask` for the Python scripts.
