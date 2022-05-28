use rsa::{pkcs8::EncodePublicKey, PaddingScheme, RsaPrivateKey, RsaPublicKey};
use sha2::{
    digest::{consts::U32, generic_array::GenericArray},
    Digest, Sha256,
};

use super::Result;
use crate::err;

/// 256 bit hash value
/// TODO: consider add a trait to caluculate hash
pub type Hash = GenericArray<u8, U32>;

#[derive(Debug, Clone)]
/// Address is the hash of the account's public key
pub struct Address(Hash);

impl Address {
    pub fn as_hash(&self) -> &Hash {
        &self.0
    }
}

#[derive(Debug, Clone)]
/// Public key used to verify signature of transaction input
pub struct PublicKey(RsaPublicKey);

#[derive(Clone)]
/// Private key used to sign transaction input
pub struct PrivateKey(RsaPrivateKey);

// TODO: google about padding scheme for signatures
const DEFAULT_PADDING_SCHEME: PaddingScheme = PaddingScheme::PKCS1v15Sign { hash: None };

impl PublicKey {
    pub fn new(inner: RsaPublicKey) -> Self {
        Self(inner)
    }

    fn inner(&self) -> &RsaPublicKey {
        &self.0
    }

    pub fn hash(&self) -> Result<Hash> {
        let mut hasher = Sha256::new();
        let encoded_pub_key = self
            .inner()
            .to_public_key_der()
            .map_err(|e| err!("failed to encode rsa public key: {}", e))?;
        hasher.update(encoded_pub_key);
        Ok(hasher.finalize())
    }

    pub fn verify(&self, expected: &Hash, signed: &[u8]) -> Result<()> {
        rsa::PublicKey::verify(&self.inner(), DEFAULT_PADDING_SCHEME, expected, signed)
            .map_err(|e| err!("failed to verify signatured hash: {}", e))
    }
}

impl PrivateKey {
    pub fn new(inner: RsaPrivateKey) -> Self {
        Self(inner)
    }

    fn inner(&self) -> &RsaPrivateKey {
        &self.0
    }

    pub fn to_public_key(&self) -> PublicKey {
        PublicKey::new(RsaPublicKey::from(self.inner()))
    }

    pub fn sign(&self, hash: &Hash) -> Result<Vec<u8>> {
        self.inner()
            .sign(DEFAULT_PADDING_SCHEME, hash)
            .map_err(|e| err!("failed to sign by private key: {}", e))
    }
}

#[test]
fn key_auth_sign_and_verify() {
    use rand;
    let mut rng = rand::thread_rng();
    let private_key = PrivateKey::new(RsaPrivateKey::new(&mut rng, 2048).expect("failed to create private_key"));
    let public_key = private_key.to_public_key();

    let hash = Hash::from_slice(&[42; 32]);
    let signed = private_key.sign(hash).expect("failed to sign the hash");
    assert_eq!(Ok(()), public_key.verify(hash, &signed));
    let other_hash = Hash::from_slice(&[10; 32]);
    let fake_sign = private_key.sign(other_hash).expect("failed to sign the fake hash");
    assert_ne!(Ok(()), public_key.verify(hash, &fake_sign));
}
