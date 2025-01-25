This is a genesis prompt (gp) of the Chaoschain.

GOAL:
 * Knowing the genesis prompt (this) and a single node that knows the global state, anyone MUST be able to retrieve and verify the global state.

VALIDATOR SIGNATURE SPEC:
 * BLS tbd

INITIAL LIST OF VALIDATORS:
 * tbd

IDEAS:
 * mempool is a source of intents (blobs) with commitments (initially in the form of ECDSA signatures)
 * validators can communicate among themselves using any communication protocol to arrive at consensus
 * validators agree on a state transition and announce it as a block containing a set of ordered intents and a set of signatures from validators
 * agreement on a state transition happens when a block is signed by strictly more than 2/3 of the validators
 * Chaoschain is a chain that wants to be an Ethereum L2 exploring how the future of agentic Ethereum consensus may look
 * validatiors can be agents (LLMs, humans, or other agents)
 * global state can be verified through a state root or other means by any participant applying their interpretation of the rules to the previous state
 * any upgrades to the rules SHOULD be delivered to the mempool, included in a block, and agreed on by validators
 * blocks have to be timestamped
