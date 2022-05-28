use sha2::{Digest, Sha256};
use std::collections::{HashMap, LinkedList};

use super::Hash;
use super::Result;
use crate::err;

#[derive(Debug, Clone)]
/// Output of transaction
pub struct TxOut {
    /// Address of the account of the receiver
    rx_addr: Hash,
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

#[derive(Debug)]
/// Block consists of some transactions
pub struct Block {
    /// The block preceding this block in the blockchain
    parent_hash: Hash,
    /// List of transactions which this block contains
    transactions: LinkedList<Transaction>,
    /// Random number used to adjust the hash of the block
    nonce: u128,
}

#[derive(Debug)]
/// Blockchain is a tree consisting of blocks
pub struct Blockchain {
    /// Transactions waiting to be wrapped in a block and pushed to the blockchain
    queued_tx: LinkedList<Transaction>,
    /// Blocks in the blockchain
    blocks: HashMap<Hash, Block>,
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
        self[0] & (0b1111_1111_u8 >> difficulty) == 0
    }
}

// TODO: Implment auto derive for hash()
impl TxOut {
    fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.rx_addr);
        hasher.update(self.amount.to_le_bytes());
        hasher.finalize()
    }
}

impl TxIn {
    fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.signature);
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
    fn new(tx: LinkedList<Transaction>, parent_hash: Hash) -> Self {
        Self {
            transactions: tx,
            parent_hash,
            nonce: 0,
        }
    }

    fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.parent_hash);
        for tx in &self.transactions {
            //hasher.update(tx.hash());
        }
        hasher.update(self.nonce.to_le_bytes());
        hasher.finalize()
    }
}

impl Blockchain {
    const TX_PER_BLOCK: usize = 16;

    fn current_hash(&self) -> Hash {
        self.trusted_last_block_hash
    }

    fn queue(&mut self, tx: Transaction) -> Result<Action> {
        self.queued_tx.push_back(tx);
        if self.queued_tx.len() >= Self::TX_PER_BLOCK {
            let tx = std::mem::take(&mut self.queued_tx);
            let new_block = Block::new(tx, self.current_hash());
            let new_block_hash = new_block.hash();
            return self.push(new_block).and_then(|()| {
                self.blocks
                    .get(&new_block_hash)
                    .ok_or_else(|| err!("hash not found in blocks"))
                    .map(Action::BroadcastBlock)
            });
        }
        Ok(Action::None)
    }

    fn push(&mut self, block: Block) -> Result<()> {
        self.verify(&block)?;
        let hash = block.hash();
        let height = 1 + self
            .blocks_height
            .get(&block.parent_hash)
            .ok_or_else(|| err!("hash not found in blocks_height"))?;
        self.blocks_height.insert(hash, height);
        self.blocks.insert(hash, block);
        if height > self.trusted_height {
            self.trusted_height = height;
            self.trusted_last_block_hash = hash;
        }
        Ok(())
    }

    fn verify(&self, block: &Block) -> Result<()> {
        if !block.hash().pow_verified(self.pow_difficulty) {
            return Err(err!("Received block does not meets difficulty of PoW"));
        }
        for tx in &block.transactions {
            for tx_in in &tx.inputs {
                todo!("implement signature verification")
            }
        }
        Ok(())
    }
}

/// Side effects caused by the blockchain to other network nodes
pub enum Action<'chain> {
    /// Do nothing
    None,
    /// Ask to verify the block and add that to the blockchains
    BroadcastBlock(&'chain Block),
}
