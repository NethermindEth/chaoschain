"""
ChaosChain Cryptography Package

Provides cryptographic primitives and utilities for ChaosChain.
Currently focused on BLS signatures using the BLS12-381 curve.
"""

from .bls import (
    generate_bls_keypair,
    bls,
    Scalar,
    SkToPk,
    bytes96_to_G2,
    BLS_MODULUS,
) 