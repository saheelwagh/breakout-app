Solana Account Model

On Solana, all data is stored in what are referred to as "accounts”. 
The way data is organized on the Solana blockchain resembles a key-value store, where each entry in the database is called an "account".



Transactions and Instructions

On Solana, we send transactions to interact with the network. Transactions include one or more instructions, each representing a specific operation to be processed. The execution logic for instructions is stored on programs deployed to the Solana network, where each program defines its own set of instructions.

Learn more about Transactions and Instructions here.

The Solana blockchain has a few different types of fees and costs that are incurred to use the network. These can be segmented into a few specific types:

Transaction Fees - A fee to have validators process transactions/instructions
Prioritization Fees - An optional fee to boost transactions processing order
Rent - A withheld balance to keep data stored on-chain


On Solana, "smart contracts" are called programs. Each program is stored in an on-chain account and contains executable code that defines specific instructions. These instructions represent the program's functionality and can be invoked by sending transactions to the network.



A Cross Program Invocation (CPI) refers to when one program invokes the instructions of another program. This mechanism allows for the composability of Solana programs.

You can think of instructions as API endpoints that a program exposes to the network and a CPI as one API internally invoking another API.

Tokens are digital assets that represent ownership over diverse categories of assets. Tokenization enables the digitalization of property rights, serving as a fundamental component for managing both fungible and non-fungible assets.

Fungible Tokens represent interchangeable and divisible assets of the same type and value (ex. USDC).
Non-fungible Tokens (NFT) represent ownership of indivisible assets (e.g. artwork).
Learn more about Tokens on Solana here.

The Solana blockchain has several different groups of validators, known as Clusters. Each serving a different purposes and containing dedicated nodes to fulfill JSON-RPC requests.

There are three primary clusters on the Solana network, with the following public endpoints:

Mainnet - https://api.mainnet-beta.solana.com (production)
Devnet - https://api.devnet.solana.com (developer experimentation)
Testnet - https://api.testnet.solana.com (validator testing)
Learn more about Clusters and Endpoints here.

https://x.com/marc_louvion/status/1910312329172189543 inspiration