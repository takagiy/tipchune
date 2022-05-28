use sha2::digest::{consts::U32, generic_array::GenericArray};

/// 256 bit hash value
pub type Hash = GenericArray<u8, U32>;

#[derive(Debug, Clone)]
/// Address is the hash of the account's public key
pub struct Address(Hash);

impl Address {
    pub fn as_hash(&self) -> &Hash {
        &self.0
    }
}
