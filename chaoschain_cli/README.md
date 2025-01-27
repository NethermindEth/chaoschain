# ChaosChain CLI Tool

A command-line interface for managing BLS keys and signed blocks in the ChaosChain ecosystem.

## Quick Start

```bash
# Install
pip install chaoschain-cli
# or with Poetry
poetry add chaoschain-cli

# Generate your keypair
chaos keygen

# Sign a message
chaos sign "Hello ChaosChain!"

# Create a block
chaos block create "My first chaotic block"
```

## Features

- **BLS Key Management**
  - Generate and store BLS12-381 keypairs
  - Sign messages with your private key
  - Verify signatures against public keys

- **Block Management**
  - Create signed blocks with arbitrary content
  - Verify block signatures and format
  - JSON output for easy scripting

- **Unix-Style Design**
  - Supports piped input/output (`echo "message" | chaos sign`)
  - JSON formatted output for scripting
  - Clear error messages
  - Command autocompletion

## Commands

```bash
# Show all commands
chaos --help

# Key management
chaos keygen              # Generate new keypair
chaos sign "message"      # Sign a message
chaos verify ...          # Verify a signature

# Block management
chaos block create "text" # Create a signed block
chaos block verify ...    # Verify a block
```

For detailed usage instructions, see the [CLI Documentation](https://github.com/tkstanczak/chaoschain/blob/main/chaoschain_cli/docs/CLI.md).

## Development

See the [API Reference](https://github.com/tkstanczak/chaoschain/blob/main/chaoschain_cli/docs/API.md) for integrating the CLI's functionality into your own applications.

## License

MIT License - see [LICENSE](../LICENSE) for details. 