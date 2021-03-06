use sha2::{Digest, Sha256};
use std::collections::HashMap;

use super::Address;
use super::Hash;
use super::PublicKey;
use super::Result;
use crate::err;

#[derive(Debug, Clone)]
/// Output of transaction
pub struct TxOut {
    /// Address of the account of the receiver
    receiver_address: Address,
    /// Value to be transferred
    amount: usize,
}

#[derive(Debug, Clone)]
/// Reference to point a transaction output from a transaction input
pub struct TxOutPtr {
    /// Hash of the transaction holding the output
    transaction_hash: Hash,
    /// Index of the output in the list of outputs of the transaction
    index: usize,
}

#[derive(Debug, Clone)]
/// Input of the transaction
pub struct TxIn {
    /// Signature to prove that the transaction creator owns `src_output`
    signature: Hash,
    /// Public key used to verify `signature`
    public_key: PublicKey,
    /// Transaction output where the input came from.
    source_output: TxOutPtr,
}

#[derive(Debug, Clone)]
/// Transaction representing a transfer of value
pub struct Transaction {
    /// Values that will be consumed after the transaction
    inputs: Vec<TxIn>,
    /// Values that will be generated after the transaction
    outputs: Vec<TxOut>,
}

#[derive(Debug, Clone)]
/// Addtional data attached to blocks
pub struct BlockDesc {
    /// The block preceding this block in the blockchain
    parent_hash: Hash,
    /// Random number used to adjust the hash of the block
    nonce: u128,
}

#[derive(Debug)]
/// Transactions contained in a block
pub struct BlockBody {
    /// List of transactions contained in a block
    transactions: Vec<Transaction>,
}

#[derive(Debug)]
/// Block consists of some transactions
pub struct Block {
    /// Addtional data attached to the block
    desc: BlockDesc,
    /// Transactions contained in the block
    body: BlockBody,
}

#[derive(Debug)]
/// Blockchain is a tree consisting of blocks
pub struct Blockchain {
    /// Transactions in the blockchain
    transactions: HashMap<Hash, Transaction>,
    /// Blocks in the blockchain
    blocks: HashMap<Hash, BlockDesc>,
    /// Height of the blocks in the tree
    block_heights: HashMap<Hash, u128>,
    /// Height of the highest block in the tree
    max_height: u128,
    /// Hash of the highest block in the tree
    max_height_block_hash: Hash,
    /// Difficulty of Proof-of-Work (number of preceding bits which must be zero)
    pow_difficulty: usize,
}

trait VerifyPow {
    fn pow_verified(&self, difficulty: usize) -> bool;
}

impl VerifyPow for Hash {
    fn pow_verified(&self, difficulty: usize) -> bool {
        assert!(difficulty <= 8);
        self[0] & !(0b1111_1111_u8 >> difficulty) == 0
    }
}

// TODO: Implment auto derive for hash()
impl TxOut {
    fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.receiver_address.as_hash());
        hasher.update(self.amount.to_le_bytes());
        hasher.finalize()
    }
}

impl TxIn {
    fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.signature);
        // TODO: forward the errors to the caller
        hasher.update(
            self.public_key
                .hash()
                .expect("failed to calculate public key hash"),
        );
        hasher.update(self.source_output.transaction_hash);
        hasher.update(self.source_output.index.to_le_bytes());
        hasher.finalize()
    }

    fn find_source<'local>(
        &self,
        local_transactions: &HashMap<Hash, &'local Transaction>,
        chain_transactions: &'local HashMap<Hash, Transaction>,
    ) -> Result<&'local TxOut> {
        local_transactions
            .get(&self.source_output.transaction_hash)
            .copied()
            .or_else(|| chain_transactions.get(&self.source_output.transaction_hash))
            .ok_or_else(|| err!("transaction output referred in transaction input does not found"))
            .map(|transaction| &transaction.outputs[self.source_output.index])
    }
}

impl Transaction {
    fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        for input in &self.inputs {
            hasher.update(input.hash());
        }
        for output in &self.outputs {
            hasher.update(output.hash());
        }
        hasher.finalize()
    }
}

impl Block {
    fn new(transactions: Vec<Transaction>, parent_hash: Hash) -> Self {
        Self {
            body: BlockBody { transactions },
            desc: BlockDesc {
                parent_hash,
                nonce: 0,
            },
        }
    }

    fn from_part(body: BlockBody, desc: BlockDesc) -> Self {
        Self { body, desc }
    }

    fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.desc.parent_hash);
        for transaction in &self.body.transactions {
            hasher.update(transaction.hash());
        }
        hasher.update(self.desc.nonce.to_le_bytes());
        hasher.finalize()
    }

    fn base_transaction(&self) -> &Transaction {
        &self.body.transactions[0]
    }
}

impl Blockchain {
    const TX_PER_BLOCK: usize = 16;

    fn current_hash(&self) -> Hash {
        self.max_height_block_hash
    }

    fn push(&mut self, block: Block) -> Result<BlockBody> {
        self.verify(&block)?;
        let hash = block.hash();
        let height = self
            .block_heights
            .get(&block.desc.parent_hash)
            .ok_or_else(|| err!("hash not found in blocks_height"))?
            .checked_add(1)
            .ok_or_else(|| err!("block height overflowed"))?;
        self.block_heights.insert(hash, height);
        self.blocks.insert(hash, block.desc);
        for transaction in &block.body.transactions {
            self.transactions
                .insert(transaction.hash(), transaction.clone());
        }
        if height > self.max_height {
            self.max_height = height;
            self.max_height_block_hash = hash;
        }
        Ok(block.body)
    }

    fn verify(&self, block: &Block) -> Result<()> {
        if !block.hash().pow_verified(self.pow_difficulty) {
            return Err(err!("Received block does not meets difficulty of PoW"));
        }
        let transactions_in_block: HashMap<Hash, &Transaction> = block
            .body
            .transactions
            .iter()
            .map(|transaction| (transaction.hash(), transaction))
            .collect();

        let mut block_input_amount = 0;
        let mut block_output_amount = 0;

        for (i, transaction) in block.body.transactions.iter().enumerate() {
            let transaction_input_amount: usize =
                transaction.inputs.iter().try_fold(0, |sum, input| {
                    let input_source =
                        input.find_source(&transactions_in_block, &self.transactions)?;

                    // Ensure that hash of public key matches with the receiver address of input_source
                    if &input.public_key.hash()? != input_source.receiver_address.as_hash() {
                        return Err(err!(
                            "public key of input does not match with address of output"
                        ));
                    }

                    input.public_key.verify(&input.hash(), &input.signature)?;
                    Ok(sum + input_source.amount)
                })?;
            let transaction_output_amount: usize =
                transaction.outputs.iter().map(|output| output.amount).sum();

            // Transaction excepting for the base transaction cannot generate new amount
            if i != 0 && transaction_output_amount > transaction_input_amount {
                return Err(err!(
                    "output amount of transaction exceeded input amount of transaction"
                ));
            }
            block_input_amount += transaction_input_amount;
            block_output_amount += transaction_output_amount;
        }
        if block_input_amount != block_output_amount {
            return Err(err!(
                "input and output amount of transactions are not balanced"
            ));
        }

        if !block.base_transaction().inputs.is_empty()
            || block.base_transaction().outputs.len() != 1
        {
            return Err(err!(
                "number of inputs and outputs of base transaction is incorrect"
            ));
        }
        Ok(())
    }
}

/// Side effects caused by the blockchain to other network nodes
pub enum Action {
    /// Do nothing
    None,
    /// Ask to verify the block and add that to the blockchains
    BroadcastBlock(Block),
}

#[test]
fn test_hash_pow_verify() {
    let hash_difficulty3 = Hash::from_slice(&[
        0b0001_1010_u8,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        10,
        11,
        12,
        13,
        14,
        15,
        16,
        17,
        18,
        19,
        20,
        21,
        22,
        23,
        24,
        25,
        26,
        27,
        28,
        29,
        30,
        31,
    ]);
    assert!(hash_difficulty3.pow_verified(0));
    assert!(hash_difficulty3.pow_verified(1));
    assert!(hash_difficulty3.pow_verified(2));
    assert!(hash_difficulty3.pow_verified(3));
    assert!(!hash_difficulty3.pow_verified(4));
}
