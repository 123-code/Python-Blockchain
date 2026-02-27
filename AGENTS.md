## Cursor Cloud specific instructions

This is a minimal educational Python project with two standalone scripts. No build system, test framework, or linter is configured.

### Scripts

- `Blockchain.py` — Blockchain class definition using Flask. Runs with `python3 Blockchain.py`. Note: the class is defined but never instantiated at module level; instantiation would hit runtime errors in `createblock()` and `ischainvalid()`.
- `Caesar-Cipher/Cipher.py` — Caesar cipher encryption. Runs with `python3 Caesar-Cipher/Cipher.py`.

### Dependencies

Only external dependency is **Flask** (`pip install flask`). The update script handles this.

### Running

```
python3 Blockchain.py
python3 Caesar-Cipher/Cipher.py
```

### Notes

- No `requirements.txt` exists; Flask is installed directly via pip.
- No test suite, linter, or CI pipeline is configured.
- `Blockchain.py` emits a `SyntaxWarning` on line 50 at import time — this is a known code issue, not an environment problem.
