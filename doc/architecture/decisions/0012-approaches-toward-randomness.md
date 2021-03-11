# 12. Generalized Randomness for Holochain applications
Date: 2018-06-5

## Status
Accepted

## Context
Distributed applications, like centralized ones, often need a source of randomness.  Having confidence and verifiability of that randomness poses particular challenges in the the distributed context. Specifically, we need a source of randomness with some of the following properties:
 1. It is outside their control or influence so it can be used for a dice roll, card deck shuffle, or something where they may have an interest in skewing the results.
 2. It is predictably reproducable, so that other nodes, whether players in a game, or validating nodes later, can reproduce the SAME random seed to reach the same random output for purposes of validation.
 3. It is generalizable. Ideally, every application won't have to build their own approach to randomness, but can take advantage of a useful underlying convention. There are pitfalls we want to help them avoid in a distributed, eventually consistent system.
 4. It doesn't require specific parties to be online to access the randomness data, so that later validators can confirm it even if parties with private entries are not online or no longer part of the network.

In the case of multiple parties wanting to generate randomness together, [cointoss](https://github.com/holochain/cointoss) provides an example of sharing of the hash of a secret which when later revealed can be combined to create a random number seed.  This method can be generalized into storing a bunch of private secrets, and publishing the hashes of those secrets, and then later revealing the secret to be combined with another party doing the same.  In cointoss this revelation happens via node-to-node communication, but in the general case it doesn't have to work that way.

Our application environment includes interactions (gossip and validation)  the combination of which are highly unpredicable (they include things like network latency, and timestamps) but verifiable after the fact. So for example, using the "first" four validation signatures on your opponent's last move as a random seed, could be one approach.

## Decision
We will:
1. Implement mixins to provide randomness generation for some usecases using the cointoss combined secret method (both n2n for single events, and dht pushed for multiple events)
2. Provide app level access to unpredictable gossip/validation events for explicit use as seeds for random number generators.

## Consequences

- We will want to have our approach reviewed as proof of the validity of these approaches  (i.e. this should get included in the security review(s))
- We need to add protocol/calls/callbacks in the network abstraction layer (See ADR 7) to gain access to this randomness.

## Some Examples and Use Cases

**Shuffle the card deck for a Poker Game:** Players take turns as dealers. Each player (including the dealer) signs a private entry to their chain with their random seed in it, then they send their seed to the dealer. The dealer combines all the seeds (including their own) to create the randomness for the shuffle and commits that to private entry. Then a player "cuts" the deck at a particular card (committing that number as a public entry). At the end of the game, the dealer can republish the combined seeds as a public entry which enables everyone to fully audit the game.

**Dice Rolls for a Backgammon Game:** I'm not sure how many dice rolls it takes to complete a backgammon game, but imagine the randomness was all generated by and for both players at the start of the game. Both players commit a private entry to their respective chains with 512 random seeds. Ideally, this would be a Merkle Tree entry type (when we've built support for that) so that you can progressively reveal leaves on that Merkle Tree via Merkle proof that they build to the final hash which was visible all along in the public header of the private entry. For each turn, both player's next random seeds are combined for the next dice roll. This ensures no single player can ever determine the outcome of the dice roll. If you've used up all the seeds before finishing the game, then both players automatically commit another block of seeds and continue play.

**Proof of Possession of Private Chain:** Suppose you want to raise the level of security/assurance for an app (such as a currency) beyond just that you have the private key to author to a chain, but that you have that source chain and its private contents as well. Similar to the backgammon example above, after chain genesis, create a private entry with 1024 randomly generated leaves on a Merkle Tree. With each new transactions, include a capabilities token that enables your counterparties to retrieve a merkle proof of the next leave (1st transaction, 1st leaf. 103rd transaction exposes 103rd leaf.) This adds a tiny amount of data to the transaction and low computing power to show you are the proper owner of the key AND it's chain.