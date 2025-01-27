# ChaosChain CLI Documentation

The ChaosChain CLI tool provides functionality for managing BLS keys and creating/verifying signed blocks.

## Installation

Using Poetry (recommended):
```bash
poetry add chaoschain-cli
```

Using pip:
```bash
pip install chaoschain-cli
```

## Key Management

### Generate a New Keypair
```bash
chaos keygen
```
This will:
- Generate a new BLS12-381 keypair
- Store it securely in `~/.chaoschain/keystore/key.json`
- Display the public key

### Show Public Key
Display your public key (useful for sharing with others):
```bash
chaos pubkey
```
This outputs your hex-encoded public key that others can use to verify your signatures.

### Sign Messages
Sign a message using your private key:
```bash
# Direct input
chaos sign "Hello ChaosChain!"

# Pipe input
echo "Hello ChaosChain!" | chaos sign

# Sign file contents
cat message.txt | chaos sign
```

Output format (JSON):
```json
{
    "message": "Hello ChaosChain!",
    "signature": "<hex-encoded BLS signature>",
    "public_key": "<hex-encoded public key>"
}
```

### Verify Signatures
Verify a BLS signature against a public key:
```bash
chaos verify "Hello ChaosChain!" <signature> <public_key>
```

### Signature Aggregation
BLS signatures can be aggregated into a single signature that proves multiple parties signed the same message:

```bash
# Aggregate multiple signatures
chaos signature aggregate <sig1> <sig2> <sig3>
```

Output format (JSON):
```json
{
    "signatures": ["<sig1>", "<sig2>", "<sig3>"],
    "aggregated": "<hex-encoded aggregated signature>"
}
```

### Verify Aggregated Signatures
Verify that multiple parties signed the same message:
```bash
chaos signature verify-aggregate "message" <pubkey1> <pubkey2> <aggregated_sig>
```

Example workflow:
```bash
# Get signatures from different keys for the same message
SIG1=$(echo "Hello" | chaos sign | jq -r .signature)
PUBKEY1=$(echo "Hello" | chaos sign | jq -r .public_key)

# (Using a different key)
SIG2=$(echo "Hello" | chaos sign | jq -r .signature)
PUBKEY2=$(echo "Hello" | chaos sign | jq -r .public_key)

# Aggregate the signatures
AGGREGATED=$(chaos signature aggregate "$SIG1" "$SIG2" | jq -r .aggregated)

# Verify the aggregated signature
chaos signature verify-aggregate "Hello" "$PUBKEY1" "$PUBKEY2" "$AGGREGATED"
```

## Block Management

### Create Blocks
Create a new signed block containing arbitrary text:
```bash
# Direct input
chaos block create "My block content"

# Pipe input
echo "My block content" | chaos block create

# Create from file
cat content.txt | chaos block create
```

Output format (JSON):
```json
{
    "block_file": "/path/to/block.json",
    "block_data": {
        "version": 1,
        "timestamp": "ISO-8601 timestamp",
        "text": "My block content",
        "signature": "<hex-encoded BLS signature>",
        "signer": "<hex-encoded public key>",
        "metadata": {
            "description": "ChaosChain block format v1: Contains arbitrary text signed by a BLS key"
        }
    }
}
```

Blocks are stored in `~/.chaoschain/blocks/` with timestamps in their filenames.

### Verify Blocks
Verify a block's signature and format:
```bash
chaos block verify path/to/block.json
```

## File Locations

- **Keystore**: `~/.chaoschain/keystore/key.json`
  - Contains your BLS private and public keys
  - Format: 
    ```json
    {
        "private_key": "<hex-encoded private key>",
        "public_key": "<hex-encoded public key>"
    }
    ```

- **Blocks**: `~/.chaoschain/blocks/`
  - Contains all created blocks
  - Filename format: `block_<timestamp>.json`

## Scripting Examples

### Sign and Verify Flow
```bash
# Sign a message and save signature
SIGNATURE=$(echo "Hello" | chaos sign | jq -r .signature)
PUBKEY=$(echo "Hello" | chaos sign | jq -r .public_key)

# Verify the signature
chaos verify "Hello" "$SIGNATURE" "$PUBKEY"
```

### Create and Verify Block Flow
```bash
# Create a block and get its path
BLOCK_FILE=$(echo "My content" | chaos block create | jq -r .block_file)

# Verify the block
chaos block verify "$BLOCK_FILE"
```

### Batch Processing
```bash
# Sign multiple messages
cat messages.txt | while read -r msg; do
    echo "$msg" | chaos sign
done

# Create blocks from multiple files
for file in content/*.txt; do
    cat "$file" | chaos block create
done
```

### Signature Aggregation Flow
```bash
# Generate signatures from multiple validators
SIGS=()
PUBKEYS=()
MESSAGE="Approve block #123"

# Each validator signs
for validator in {1..3}; do
    # (Using different keys for each validator)
    RESULT=$(echo "$MESSAGE" | chaos sign)
    SIGS+=("$(echo $RESULT | jq -r .signature)")
    PUBKEYS+=("$(echo $RESULT | jq -r .public_key)")
done

# Aggregate all signatures
AGGREGATED=$(chaos signature aggregate "${SIGS[@]}" | jq -r .aggregated)

# Verify the aggregated signature
chaos signature verify-aggregate "$MESSAGE" "${PUBKEYS[@]}" "$AGGREGATED"
```

## Error Handling

The CLI will provide clear error messages for common issues:
- Missing keypair: "No keypair found. Generate one with 'chaos keygen'"
- Invalid signatures: "❌ Signature is invalid"
- Missing block fields: "❌ Invalid block format: missing <field>"
- Invalid hex encoding: Detailed error message 