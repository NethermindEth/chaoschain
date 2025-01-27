"""
BLS Cryptography Module for ChaosChain

This module provides BLS signature functionality using BLS12-381 curve.
It combines multiple BLS implementations for optimal performance and security.

Author: Tomasz K Stanczak <tkstanczak@demerzel.co>
License: MIT
"""

import secrets
from typing import Any

from py_ecc.bls import G2ProofOfPossession as py_ecc_bls
from py_ecc.bls.g2_primitives import signature_to_G2 as _signature_to_G2
from py_ecc.utils import prime_field_inv as py_ecc_prime_field_inv
from py_ecc.optimized_bls12_381 import (  # noqa: F401
    G1 as py_ecc_G1,
    G2 as py_ecc_G2,
    Z1 as py_ecc_Z1,
    Z2 as py_ecc_Z2,
    add as py_ecc_add,
    multiply as py_ecc_mul,
    neg as py_ecc_neg,
    pairing as py_ecc_pairing,
    final_exponentiate as py_ecc_final_exponentiate,
    FQ12 as py_ecc_GT,
    FQ,
    FQ2,
)
from py_ecc.bls.g2_primitives import (  # noqa: F401
    curve_order as BLS_MODULUS,
    G1_to_pubkey as py_ecc_G1_to_bytes48,
    pubkey_to_G1 as py_ecc_bytes48_to_G1,
    G2_to_signature as py_ecc_G2_to_bytes96,
    signature_to_G2 as py_ecc_bytes96_to_G2,
)
from py_arkworks_bls12381 import (
    G1Point as arkworks_G1,
    G2Point as arkworks_G2,
    Scalar as arkworks_Scalar,
    GT as arkworks_GT,
)

import milagro_bls_binding as milagro_bls  # noqa: F401 for BLS switching option
import py_arkworks_bls12381 as arkworks_bls  # noqa: F401 for BLS switching option

class fastest_bls:
    """
    Fastest BLS implementation combining multiple libraries.
    Uses the most efficient implementation for each operation.
    """
    G1 = arkworks_G1
    G2 = arkworks_G2
    Scalar = arkworks_Scalar
    GT = arkworks_GT
    _AggregatePKs = milagro_bls._AggregatePKs
    Sign = milagro_bls.Sign
    Verify = milagro_bls.Verify
    Aggregate = milagro_bls.Aggregate
    AggregateVerify = milagro_bls.AggregateVerify
    FastAggregateVerify = milagro_bls.FastAggregateVerify
    SkToPk = milagro_bls.SkToPk

# Flag to make BLS active or not. Used for testing, do not ignore BLS in production unless you know what you are doing.
bls_active = True

# Default to fastest_bls
bls = fastest_bls
Scalar = fastest_bls.Scalar

def SkToPk(SK: int) -> bytes:
    """Convert a private key to a public key.
    
    Args:
        SK: Private key as an integer
        
    Returns:
        bytes: Public key as compressed bytes
    """
    if bls == py_ecc_bls or bls == arkworks_bls:  # no signature API in arkworks
        return py_ecc_bls.SkToPk(SK)
    else:
        return bls.SkToPk(SK.to_bytes(32, 'big'))

def bytes96_to_G2(bytes96: bytes) -> Any:
    """
    Deserializes a purported compressed serialized point in G2.
    
    Args:
        bytes96: Compressed G2 point
        
    Returns:
        Any: Deserialized G2 point (either arkworks_G2 or py_ecc G2 point)
        
    Notes:
        - No subgroup checks are performed
        - If the bytearray is not a valid serialization of a point in G2,
          then this method will raise an exception
    """
    if bls == arkworks_bls or bls == fastest_bls:
        return arkworks_G2.from_compressed_bytes_unchecked(bytes96)
    return py_ecc_bytes96_to_G2(bytes96)

def generate_bls_keypair() -> tuple[int, bytes]:
    """
    Generates a BLS12-381 private/public key pair.
    
    Returns:
        tuple[int, bytes]: (private_key_int, public_key_bytes)
        - private_key_int: Integer representation of private key
        - public_key_bytes: Compressed G1 point (48 bytes)
    """
    # 1. Generate a random 32-byte private key and reduce it mod curve_order
    private_key = int.from_bytes(secrets.token_bytes(32), byteorder='big') % BLS_MODULUS
    
    # 2. Derive the public key (a point on G1) and serialize
    public_key = SkToPk(private_key)
    
    return private_key, public_key 