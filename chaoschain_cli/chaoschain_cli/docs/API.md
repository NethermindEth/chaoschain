# ChaosChain CLI API Reference

This document describes the Python API for the ChaosChain CLI tool. This is useful if you want to integrate BLS signing functionality into your own Python applications.

## Cryptography Module

### BLS Signatures (`chaoschain_cli.crypto.bls`)

#### `generate_bls_keypair() -> tuple[int, bytes]`
Generate a new BLS12-381 keypair.

```python
from chaoschain_cli.crypto import generate_bls_keypair

private_key, public_key = generate_bls_keypair()
# private_key: Integer representation of private key
# public_key: Compressed G1 point (48 bytes)
```

#### `SkToPk(SK: int) -> bytes`
Convert a private key to a public key.

```python
from chaoschain_cli.crypto import SkToPk

public_key = SkToPk(private_key)
# Returns compressed G1 point (48 bytes)
```

#### `bytes96_to_G2(bytes96: bytes) -> G2Point`
Deserialize a compressed G2 point.

```python
from chaoschain_cli.crypto import bytes96_to_G2

g2_point = bytes96_to_G2(signature_bytes)
# Returns deserialized G2 point
```

### BLS Implementation (`chaoschain_cli.crypto.bls.fastest_bls`)

The `fastest_bls` class combines multiple BLS implementations for optimal performance:

```python
from chaoschain_cli.crypto import bls

# Sign a message
signature = bls.Sign(private_key_bytes, message_bytes)

# Verify a signature
is_valid = bls.Verify(public_key_bytes, message_bytes, signature_bytes)

# Aggregate multiple signatures
aggregated_sig = bls.Aggregate([sig1, sig2, sig3])

# Verify aggregated signature
is_valid = bls.AggregateVerify(
    [pk1, pk2, pk3],
    [msg1, msg2, msg3],
    aggregated_sig
)

# Fast aggregate verify (same message)
is_valid = bls.FastAggregateVerify(
    [pk1, pk2, pk3],
    message,
    signature
)
```

## CLI Module

### Command Line Interface (`chaoschain_cli.cli.bls_tool`)

The CLI module provides the command-line interface. You can also use its functions directly:

```python
from chaoschain_cli.cli.bls_tool import (
    ensure_dirs,
    load_keypair,
    KEYSTORE_DIR,
    BLOCKS_DIR
)

# Ensure required directories exist
ensure_dirs()

# Load existing keypair
private_key, public_key = load_keypair()
```

### Constants

```python
from chaoschain_cli.cli.bls_tool import KEYSTORE_DIR, BLOCKS_DIR

# Default keystore location
# ~/.chaoschain/keystore/key.json
print(KEYSTORE_DIR)

# Default blocks directory
# ~/.chaoschain/blocks/
print(BLOCKS_DIR)
```

## File Formats

### Keystore File
Location: `~/.chaoschain/keystore/key.json`
```json
{
    "private_key": "<hex-encoded private key>",
    "public_key": "<hex-encoded public key>"
}
```

### Block File
Location: `~/.chaoschain/blocks/block_<timestamp>.json`
```json
{
    "version": 1,
    "timestamp": "ISO-8601 timestamp",
    "text": "Block content",
    "signature": "<hex-encoded BLS signature>",
    "signer": "<hex-encoded public key>",
    "metadata": {
        "description": "ChaosChain block format v1: Contains arbitrary text signed by a BLS key"
    }
}
```

## Example Usage

### Key Management
```python
from chaoschain_cli.crypto import generate_bls_keypair, bls
import json

# Generate new keypair
private_key, public_key = generate_bls_keypair()

# Save to keystore
keystore = {
    "private_key": hex(private_key),
    "public_key": public_key.hex()
}
with open("key.json", "w") as f:
    json.dump(keystore, f, indent=2)

# Sign a message
message = b"Hello ChaosChain!"
signature = bls.Sign(
    private_key.to_bytes(32, 'big'),
    message
)

# Verify the signature
is_valid = bls.Verify(
    public_key,
    message,
    signature
)
```

### Block Creation
```python
import datetime
import json

# Create a block
block_data = {
    "version": 1,
    "timestamp": datetime.datetime.utcnow().isoformat(),
    "text": "My block content",
    "signer": public_key.hex(),
    "metadata": {
        "description": "ChaosChain block format v1"
    }
}

# Sign the text
signature = bls.Sign(
    private_key.to_bytes(32, 'big'),
    block_data["text"].encode('utf-8')
)
block_data["signature"] = signature.hex()

# Save block
with open("block.json", "w") as f:
    json.dump(block_data, f, indent=2)
``` 