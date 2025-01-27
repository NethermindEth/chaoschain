"""
ChaosChain BLS CLI Tool

A command-line interface for managing BLS keys and signed blocks in the ChaosChain ecosystem.
Provides functionality for key generation, message signing, and block creation/verification.

Author: Tomasz K Stanczak <tkstanczak@demerzel.co>
License: MIT
Version: 0.1.0
"""

import click
import json
import os
import sys
import datetime
from pathlib import Path
from typing import Optional

from chaoschain_cli.crypto import generate_bls_keypair, bls, Scalar

__version__ = "0.1.0"
__author__ = "nethermind"
__license__ = "MIT"

# Constants
KEYSTORE_DIR = Path.home() / ".chaoschain" / "keystore"
BLOCKS_DIR = Path.home() / ".chaoschain" / "blocks"

def ensure_dirs():
    """Ensure required directories exist"""
    KEYSTORE_DIR.mkdir(parents=True, exist_ok=True)
    BLOCKS_DIR.mkdir(parents=True, exist_ok=True)

def load_keypair() -> tuple[int, bytes]:
    """Load the keypair from keystore"""
    keyfile = KEYSTORE_DIR / "key.json"
    if not keyfile.exists():
        click.echo("No keypair found. Generate one with 'chaos keygen'", err=True)
        raise click.Abort()
    
    with open(keyfile) as f:
        data = json.load(f)
        return int(data["private_key"], 16), bytes.fromhex(data["public_key"])

@click.group()
@click.version_option(version=__version__)
def cli():
    """ChaosChain CLI - BLS key and block management tool
    
    Start with 'chaos keygen' to generate your keypair.
    Run 'chaos --help' to see all commands.
    """
    # Show welcome message on first run
    if not KEYSTORE_DIR.exists() and not BLOCKS_DIR.exists():
        click.echo(click.style("\n🌪  Welcome to ChaosChain! 🌪\n", fg="green", bold=True))
        click.echo("Quick start:")
        click.echo("  chaos keygen              - Generate your BLS keypair")
        click.echo("  chaos sign \"message\"       - Sign a message")
        click.echo("  chaos block create \"text\" - Create a signed block")
        click.echo("\nFor more information:")
        click.echo("  chaos --help              - Show all commands")
        click.echo("  chaos COMMAND --help      - Show help for a command\n")
    
    ensure_dirs()

@cli.command()
def keygen():
    """Generate a new BLS keypair"""
    keyfile = KEYSTORE_DIR / "key.json"
    
    if keyfile.exists():
        if not click.confirm("Keypair already exists. Overwrite?"):
            return
    
    private_key, public_key = generate_bls_keypair()
    
    # Store keys securely
    with open(keyfile, "w") as f:
        json.dump({
            "private_key": hex(private_key),
            "public_key": public_key.hex()
        }, f, indent=2)
    
    click.echo(f"Generated new keypair")
    click.echo(f"Public key: {public_key.hex()}")

@cli.command()
@click.argument('message', required=False)
def sign(message: str = None):
    """Sign a message with your private key. If no message is provided, reads from stdin."""
    private_key, public_key = load_keypair()
    
    # If no message provided, read from stdin
    if message is None:
        if not sys.stdin.isatty():  # Check if input is being piped
            message = sys.stdin.read().strip()
        else:
            click.echo("No message provided and no input piped. Use either:", err=True)
            click.echo("  chaos sign \"your message\"", err=True)
            click.echo("  echo \"your message\" | chaos sign", err=True)
            raise click.Abort()
    
    # Convert message to bytes and sign
    message_bytes = message.encode('utf-8')
    signature = bls.Sign(private_key.to_bytes(32, 'big'), message_bytes)
    
    # Output in a format that's easy to parse and pipe
    result = {
        "message": message,
        "signature": signature.hex(),
        "public_key": public_key.hex()
    }
    click.echo(json.dumps(result))

@cli.command()
@click.argument('message')
@click.argument('signature')
@click.argument('public_key')
def verify(message: str, signature: str, public_key: str):
    """Verify a signature against a public key"""
    try:
        message_bytes = message.encode('utf-8')
        signature_bytes = bytes.fromhex(signature)
        pubkey_bytes = bytes.fromhex(public_key)
        
        is_valid = bls.Verify(pubkey_bytes, message_bytes, signature_bytes)
        
        if is_valid:
            click.echo("✅ Signature is valid")
        else:
            click.echo("❌ Signature is invalid")
            
    except Exception as e:
        click.echo(f"Error verifying signature: {e}", err=True)
        raise click.Abort()

@cli.group()
def block():
    """Block management commands"""
    pass

@block.command(name="create")
@click.argument('text', required=False)
def create_block(text: str = None):
    """Create a new signed block with given text. If no text is provided, reads from stdin."""
    private_key, public_key = load_keypair()
    
    # If no text provided, read from stdin
    if text is None:
        if not sys.stdin.isatty():  # Check if input is being piped
            text = sys.stdin.read().strip()
        else:
            click.echo("No text provided and no input piped. Use either:", err=True)
            click.echo("  chaos block create \"your text\"", err=True)
            click.echo("  echo \"your text\" | chaos block create", err=True)
            raise click.Abort()
    
    # Create block
    block_data = {
        "version": 1,
        "timestamp": datetime.datetime.utcnow().isoformat(),
        "text": text,
        "signer": public_key.hex(),
        "metadata": {
            "description": "ChaosChain block format v1: Contains arbitrary text signed by a BLS key"
        }
    }
    
    # Sign the text
    signature = bls.Sign(private_key.to_bytes(32, 'big'), text.encode('utf-8'))
    block_data["signature"] = signature.hex()
    
    # Save block
    block_file = BLOCKS_DIR / f"block_{block_data['timestamp'].replace(':', '-')}.json"
    with open(block_file, "w") as f:
        json.dump(block_data, f, indent=2)
    
    # Output in a format that's easy to parse and pipe
    result = {
        "block_file": str(block_file),
        "block_data": block_data
    }
    click.echo(json.dumps(result))

@block.command(name="verify")
@click.argument('block_file', type=click.Path(exists=True))
def verify_block(block_file: str):
    """Verify a block's signature"""
    try:
        with open(block_file) as f:
            block_data = json.load(f)
        
        # Basic format validation
        required_fields = ["version", "timestamp", "text", "signature", "signer"]
        for field in required_fields:
            if field not in block_data:
                click.echo(f"❌ Invalid block format: missing {field}", err=True)
                return
        
        # Verify signature
        text_bytes = block_data["text"].encode('utf-8')
        signature_bytes = bytes.fromhex(block_data["signature"])
        pubkey_bytes = bytes.fromhex(block_data["signer"])
        
        is_valid = bls.Verify(pubkey_bytes, text_bytes, signature_bytes)
        
        if is_valid:
            click.echo("✅ Block signature is valid")
            click.echo(f"Signer: {block_data['signer']}")
            click.echo(f"Timestamp: {block_data['timestamp']}")
            click.echo(f"Text: {block_data['text']}")
        else:
            click.echo("❌ Block signature is invalid")
            
    except Exception as e:
        click.echo(f"Error verifying block: {e}", err=True)
        raise click.Abort()

@cli.group()
def signature():
    """Signature management commands"""
    pass

@signature.command(name="aggregate")
@click.argument('signatures', nargs=-1)
def aggregate_signatures(signatures: list[str]):
    """Aggregate multiple BLS signatures into one.
    
    Example: chaos signature aggregate <sig1> <sig2> <sig3>
    """
    try:
        # Convert hex signatures to bytes
        sig_bytes = [bytes.fromhex(sig) for sig in signatures]
        
        # Aggregate signatures
        aggregated = bls.Aggregate(sig_bytes)
        
        # Output result
        result = {
            "signatures": signatures,
            "aggregated": aggregated.hex()
        }
        click.echo(json.dumps(result))
            
    except Exception as e:
        click.echo(f"Error aggregating signatures: {e}", err=True)
        raise click.Abort()

@signature.command(name="verify-aggregate")
@click.argument('message')
@click.argument('public_keys', nargs=-1)
@click.argument('aggregated_signature')
def verify_aggregate(message: str, public_keys: list[str], aggregated_signature: str):
    """Verify an aggregated signature against multiple public keys.
    All signers must have signed the same message.
    
    Example: chaos signature verify-aggregate "message" <pubkey1> <pubkey2> <aggregated_sig>
    """
    try:
        # Convert inputs to bytes
        message_bytes = message.encode('utf-8')
        pubkey_bytes = [bytes.fromhex(pk) for pk in public_keys]
        sig_bytes = bytes.fromhex(aggregated_signature)
        
        # Verify aggregated signature
        is_valid = bls.FastAggregateVerify(pubkey_bytes, message_bytes, sig_bytes)
        
        if is_valid:
            click.echo("✅ Aggregated signature is valid")
        else:
            click.echo("❌ Aggregated signature is invalid")
            
    except Exception as e:
        click.echo(f"Error verifying aggregated signature: {e}", err=True)
        raise click.Abort()

@cli.command()
def pubkey():
    """Show your public key"""
    _, public_key = load_keypair()
    click.echo(public_key.hex())

if __name__ == '__main__':
    cli() 