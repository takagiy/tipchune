use sha2::digest::{consts::U32, generic_array::GenericArray};

pub type Hash = GenericArray<u8, U32>;
