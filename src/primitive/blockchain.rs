use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::collections::HashSet;

use super::Address;
use super::Hash;
use super::PublicKey;
use super::Result;
use crate::err;

#[derive(Debug, Clone)]
/// Output of transaction
pub struct TxOut {
    /// Address of the account of the receiver
    rx_addr: Address,
    /// Value to be transferred
    amount: usize,
}

#[derive(Debug, Clone)]
/// Reference to point a transaction output from a transaction input
pub struct TxOutPtr {
    /// Hash of the transaction holding the output
    tx_hash: Hash,
    /// Index of the output in the list of outputs of the transaction
    output_idx: usize,
}

#[derive(Debug, Clone)]
/// Input of the transaction
pub struct TxIn {
    /// Signature to prove that the transaction creator owns `src_output`
    signature: Hash,
    /// Public key used to verify `signature`
    public_key: PublicKey,
    /// Transaction output where the input came from.
    src_output: TxOutPtr,
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
    /// Transactions waiting to be wrapped in a block and pushed to the blockchain
    queued_tx: Vec<Transaction>,
    /// Transactions in the blockchain
    transactions: HashMap<Hash, Transaction>,
    /// Blocks in the blockchain
    blocks: HashMap<Hash, BlockDesc>,
    /// Height of the blocks in the tree
    blocks_height: HashMap<Hash, u128>,
    /// Height of the highest block in the tree
    trusted_height: u128,
    /// Hash of the highest block in the tree
    trusted_last_block_hash: Hash,
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
        hasher.update(self.rx_addr.as_hash());
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
        hasher.update(self.src_output.tx_hash);
        hasher.update(self.src_output.output_idx.to_le_bytes());
        hasher.finalize()
    }
}

impl Transaction {
    fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        for tx_in in &self.inputs {
            hasher.update(tx_in.hash());
        }
        for tx_out in &self.outputs {
            hasher.update(tx_out.hash());
        }
        hasher.finalize()
    }
}

impl Block {
    fn new(tx: Vec<Transaction>, parent_hash: Hash) -> Self {
        Self {
            body: BlockBody { transactions: tx },
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
        for tx in &self.body.transactions {
            hasher.update(tx.hash());
        }
        hasher.update(self.desc.nonce.to_le_bytes());
        hasher.finalize()
    }
}

impl Blockchain {
    const TX_PER_BLOCK: usize = 16;

    fn current_hash(&self) -> Hash {
        self.trusted_last_block_hash
    }

    fn queue(&mut self, tx: Transaction) -> Result<Action> {
        self.queued_tx.push(tx);
        if self.queued_tx.len() >= Self::TX_PER_BLOCK {
            let tx = std::mem::replace(
                &mut self.queued_tx,
                Vec::with_capacity(Blockchain::TX_PER_BLOCK),
            );
            let new_block = Block::new(tx, self.current_hash());
            let desc = new_block.desc.clone();
            return self
                .push(new_block)
                .map(|body| Action::BroadcastBlock(Block::from_part(body, desc)));
        }
        Ok(Action::None)
    }

    fn push(&mut self, block: Block) -> Result<BlockBody> {
        self.verify(&block)?;
        let hash = block.hash();
        let height = self
            .blocks_height
            .get(&block.desc.parent_hash)
            .ok_or_else(|| err!("hash not found in blocks_height"))?
            .checked_add(1)
            .ok_or_else(|| err!("block height overflowed"))?;
        self.blocks_height.insert(hash, height);
        self.blocks.insert(hash, block.desc);
        for tx in &block.body.transactions {
            self.transactions.insert(tx.hash(), tx.clone());
        }
        if height > self.trusted_height {
            self.trusted_height = height;
            self.trusted_last_block_hash = hash;
        }
        Ok(block.body)
    }

    fn verify(&self, block: &Block) -> Result<()> {
        if !block.hash().pow_verified(self.pow_difficulty) {
            return Err(err!("Received block does not meets difficulty of PoW"));
        }
        let tx_in_block: HashMap<Hash, &Transaction> = block
            .body
            .transactions
            .iter()
            .map(|tx| (tx.hash(), tx))
            .collect();
        for tx in &block.body.transactions {
            for tx_in in &tx.inputs {
                tx_in.public_key.verify(&tx_in.hash(), &tx_in.signature)?;
                let source_tx = tx_in_block
                    .get(&tx_in.src_output.tx_hash)
                    .copied()
                    .or_else(|| self.transactions.get(&tx_in.src_output.tx_hash))
                    .ok_or_else(|| {
                        err!("transaction output referred in transaction input does not found")
                    })?;
                let source_tx_output = &source_tx.outputs[tx_in.src_output.output_idx];

                // Ensure that hash of public key matches with the rx address of src_output
                if &tx_in.public_key.hash()? != source_tx_output.rx_addr.as_hash() {
                    return Err(err!(
                        "public key of input does not match with address of output"
                    ));
                }
                // TODO: ensure that tx input amount, tx output amount and mining reward are
                //       balanced
            }
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
