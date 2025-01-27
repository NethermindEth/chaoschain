This is a genesis prompt (gp) of the Chaoschain.

GOAL:
 * Knowing the genesis prompt (this) and a single node that knows the global state, anyone MUST be able to retrieve and verify the global state.

VALIDATOR SIGNATURE SPEC:
 * BLS12-381 curve
 * Public keys: compressed G1 points (48 bytes)
 * Private keys: integers mod BLS_MODULUS
 * Signatures: compressed G2 points (96 bytes)
 * Milagro implementation for Sign/Verify/Aggregate
 * FastAggregateVerify for 2/3+ threshold validation

INITIAL LIST OF VALIDATORS:
 * Jorik (jorikschellekens) 81ced15e9fbdd86eea101ad8bbf0eb7bcecb38b4af1b8e20cb151558b4e1f8af0dfe30e3ead35de37d9c252281b39558
 * Michal (mpzajac) 8af5ec565d6660db2ca15d1454f80d724c736e360e0b19a4946acb6b06e90489223967d7f206bafb841ab87bce465125
 * Tomasz (tkstanczak) [PUBLIC_KEY]
 * ...

IDEAS:
 * memepool is a source of intents (blobs) with commitments (initially in the form of ECDSA secp256k1 signatures)
 * validators can communicate among themselves using any communication protocol to arrive at consensus
 * validators agree on a state transition and announce it as a block containing a set of ordered intents and a set of signatures from validators
 * agreement on a state transition happens when a block is signed by strictly more than 2/3 of the validators
 * Chaoschain is a chain that wants to be an Ethereum L2 exploring how the future of agentic Ethereum consensus may look
 * validators can be agents (LLMs, humans, or other agents)
 * global state can be verified through a state root or other means by any participant applying their interpretation of the rules to the previous state
 * any upgrades to the rules SHOULD be delivered to the memepool, included in a block, and agreed on by validators
 * blocks have to be timestamped
