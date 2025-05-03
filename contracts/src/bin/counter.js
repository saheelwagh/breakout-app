// --- Prerequisites ---
// 1. Install the Solana web3 library: `npm install @solana/web3.js`
// 2. You need a funded keypair (the 'payer') on the chosen network (e.g., Devnet).
//    You can airdrop SOL to your keypair address on Devnet/Testnet using the Solana CLI:
//    `solana airdrop 2 <YOUR_WALLET_ADDRESS> --url devnet`
// 3. Run this code using Node.js: `node your_script_name.js`

// Import necessary classes from the Solana web3 library
const {
    Connection,
    Keypair,        // Represents an account keypair (public + private key)
    LAMPORTS_PER_SOL, // Constant for converting SOL to lamports
    PublicKey,      // Represents an account address
    SystemProgram,  // Native program for creating accounts and transferring SOL
    Transaction,    // Represents a Solana transaction
    sendAndConfirmTransaction, // Utility function to send and confirm transactions
} = require("@solana/web3.js");

// --- Configuration ---

// 1. Establish connection to the Solana cluster (Devnet in this case)
// You can replace this with Mainnet-beta, Testnet, or a local validator URL
const connection = new Connection("https://api.devnet.solana.com", "confirmed");
console.log("✅ Connected to Devnet");

// 2. Define the Payer Account
// IMPORTANT: Replace this with your actual secret key array or load it securely.
// NEVER commit secret keys directly into your code in production.
// This keypair needs to have SOL on Devnet to pay for transaction fees and rent.
// Example: Uint8Array.from([ ... your secret key bytes ... ])
// For demonstration, we generate a new one (won't work unless funded externally)
const payer = Keypair.generate(); // In a real scenario, load your funded keypair
console.log(`🔑 Payer Account: ${payer.publicKey.toBase58()}`);
// ** ACTION REQUIRED: You would typically load your keypair here, e.g., from a file **
// const payer = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(require('fs').readFileSync('/path/to/your/keypair.json', 'utf-8'))));

// --- Main Logic ---

const createAccount = async () => {
    try {
        // Request an airdrop to the payer account FOR DEMONSTRATION ONLY
        // In a real app, the payer must already be funded.
        // Airdrops are only available on Devnet/Testnet.
        console.log(`Requesting airdrop for payer ${payer.publicKey.toBase58()}...`);
        const airdropSignature = await connection.requestAirdrop(
            payer.publicKey,
            1 * LAMPORTS_PER_SOL // Request 1 SOL
        );
        await connection.confirmTransaction(airdropSignature, "confirmed");
        console.log("✅ Airdrop successful");

        // 3. Generate a new Keypair for the account we want to create
        const newAccount = Keypair.generate();
        console.log(`🔑 New Account Address: ${newAccount.publicKey.toBase58()}`);

        // 4. Calculate Rent-Exempt Minimum Balance
        // We need to determine the minimum SOL required for the account to be rent-exempt.
        // This depends on the amount of data space allocated (`space`).
        // For this example, we allocate 0 space (just a basic account).
        const space = 0; // Size of data in bytes for the new account
        const rentExemptionMinimum = await connection.getMinimumBalanceForRentExemption(space);
        console.log(`💰 Minimum balance for rent exemption (0 space): ${rentExemptionMinimum} Lamports`);

        // 5. Create the Transaction
        const transaction = new Transaction();

        // 6. Add the "Create Account" instruction
        transaction.add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,          // The account paying for the transaction and rent
                newAccountPubkey: newAccount.publicKey, // The public key of the new account being created
                lamports: rentExemptionMinimum,       // Amount of SOL (in lamports) to transfer to the new account
                space: space,                         // Amount of space (in bytes) to allocate for the account's data
                programId: SystemProgram.programId,   // The owner of the new account (SystemProgram for basic accounts)
            })
        );

        console.log(`⏳ Creating account ${newAccount.publicKey.toBase58()}...`);

        // 7. Sign and Send the Transaction
        // The transaction needs to be signed by the payer and the new account's keypair
        const signature = await sendAndConfirmTransaction(
            connection,
            transaction,
            [payer, newAccount] // Signers: Payer pays, NewAccount proves ownership of the key
        );

        console.log(`✅ Transaction successful! Signature: ${signature}`);
        console.log(`   Account Created: ${newAccount.publicKey.toBase58()}`);

        // 8. Verify Account Creation (Optional)
        const accountInfo = await connection.getAccountInfo(newAccount.publicKey);
        if (accountInfo) {
            console.log("\n🔍 Verification:");
            console.log(`   Owner: ${accountInfo.owner.toBase58()}`); // Should be SystemProgram
            console.log(`   Lamports: ${accountInfo.lamports}`);      // Should match rentExemptionMinimum
            console.log(`   Executable: ${accountInfo.executable}`);  // Should be false
            console.log(`   Data size: ${accountInfo.data.length}`); // Should be 0 (or match 'space')
        } else {
            console.log("❌ Verification failed: Account not found.");
        }

    } catch (error) {
        console.error("❌ Error creating account:", error);
        if (error.logs) {
            console.error("Transaction Logs:", error.logs);
        }
    }
};

// Run the function
createAccount();


// Explanation of the Code:**

// 1.  **Prerequisites & Setup:** Imports the necessary tools from `@solana/web3.js` and sets up a connection to the Solana Devnet. It also defines a `payer` keypair (which needs funding).
// 2.  **Generate New Keypair:** `Keypair.generate()` creates a unique public/private key pair for the account we intend to create on-chain. The public key becomes the account's address.
// 3.  **Calculate Rent:** `connection.getMinimumBalanceForRentExemption(space)` asks the network for the minimum lamports required for an account with the specified `space` (0 bytes in this case) to avoid paying rent.
// 4.  **Create Instruction:** `SystemProgram.createAccount({...})` builds the instruction needed to create the account. It specifies:
//     * `fromPubkey`: Who pays the fees and provides the initial lamports.
//     * `newAccountPubkey`: The address of the account to be created.
//     * `lamports`: The initial balance for the new account (set to the rent-exempt minimum).
//     * `space`: How much data storage to allocate (0 bytes).
//     * `programId`: The **owner** of the new account. Here, `SystemProgram.programId` makes it a standard, basic account owned by the system. To create a data account for *your* program, you would put *your program's ID* here.
// 5.  **Create & Send Transaction:** The instruction is added to a `Transaction`. This transaction is then signed by the `payer` (to authorize fee payment and lamport transfer) and the `newAccount` keypair (to prove control over the new address). `sendAndConfirmTransaction` sends it to the network and waits for confirmation.
// 6.  **Verification:** After confirmation, `connection.getAccountInfo` fetches the details of the newly created account directly from the blockchain, confirming its existence, owner, balance, and other properties.

// This example clearly shows the client-side steps involved in bringing a new account into existence on Solana, highlighting the roles of addresses, rent, ownership (by the System Program in this case), and the transaction proce