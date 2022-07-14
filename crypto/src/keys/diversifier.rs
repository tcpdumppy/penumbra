use std::convert::{TryFrom, TryInto};

use aes::Aes256;
use anyhow::anyhow;
use ark_ff::PrimeField;
use derivative::Derivative;
use fpe::ff1;
use penumbra_proto::{crypto as pb, Protobuf};
use serde::{Deserialize, Serialize};

use crate::Fq;

pub const DIVERSIFIER_LEN_BYTES: usize = 11;

#[derive(Copy, Clone, PartialEq, Eq, Derivative, Serialize, Deserialize)]
#[derivative(Debug)]
#[serde(try_from = "pb::Diversifier", into = "pb::Diversifier")]
pub struct Diversifier(
    #[derivative(Debug(bound = "", format_with = "crate::fmt_hex"))] pub [u8; DIVERSIFIER_LEN_BYTES],
);

impl Diversifier {
    /// Generate the diversified basepoint associated to this diversifier.
    pub fn diversified_generator(&self) -> decaf377::Element {
        let hash = blake2b_simd::Params::new()
            .personal(b"Penumbra_Divrsfy")
            .hash(&self.0);

        decaf377::Element::map_to_group_cdh(&Fq::from_le_bytes_mod_order(hash.as_bytes()))
    }
}

impl AsRef<[u8; DIVERSIFIER_LEN_BYTES]> for Diversifier {
    fn as_ref(&self) -> &[u8; DIVERSIFIER_LEN_BYTES] {
        &self.0
    }
}

impl TryFrom<&[u8]> for Diversifier {
    type Error = anyhow::Error;

    fn try_from(slice: &[u8]) -> Result<Diversifier, Self::Error> {
        if slice.len() != DIVERSIFIER_LEN_BYTES {
            return Err(anyhow!(
                "diversifier must be 11 bytes, got {:?}",
                slice.len()
            ));
        }

        let mut bytes = [0u8; DIVERSIFIER_LEN_BYTES];
        bytes.copy_from_slice(&slice[0..11]);
        Ok(Diversifier(bytes))
    }
}

impl Protobuf<pb::Diversifier> for Diversifier {}

impl From<Diversifier> for pb::Diversifier {
    fn from(d: Diversifier) -> pb::Diversifier {
        pb::Diversifier {
            inner: d.as_ref().to_vec(),
        }
    }
}

impl TryFrom<pb::Diversifier> for Diversifier {
    type Error = anyhow::Error;

    fn try_from(d: pb::Diversifier) -> Result<Diversifier, Self::Error> {
        d.inner.as_slice().try_into()
    }
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct DiversifierKey(
    #[derivative(Debug(bound = "", format_with = "crate::fmt_hex"))] pub(super) [u8; 32],
);

impl DiversifierKey {
    pub fn diversifier_for_index(&self, index: &AddressIndex) -> Diversifier {
        let enc_index = ff1::FF1::<Aes256>::new(&self.0, 2)
            .expect("radix 2 is in range")
            .encrypt(b"", &ff1::BinaryNumeralString::from_bytes_le(&index.0))
            .expect("binary string is the configured radix (2)");

        let mut diversifier_bytes = [0; 11];
        diversifier_bytes.copy_from_slice(&enc_index.to_bytes_le());
        Diversifier(diversifier_bytes)
    }

    pub fn index_for_diversifier(&self, diversifier: &Diversifier) -> AddressIndex {
        let index = ff1::FF1::<Aes256>::new(&self.0, 2)
            .expect("radix 2 is in range")
            .decrypt(
                b"",
                &ff1::BinaryNumeralString::from_bytes_le(&diversifier.0),
            )
            .expect("binary string is in the configured radix (2)");

        let mut index_bytes = [0; 11];
        index_bytes.copy_from_slice(&index.to_bytes_le());
        AddressIndex(index_bytes)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Derivative, Serialize, Deserialize)]
#[serde(try_from = "pb::AddressIndex", into = "pb::AddressIndex")]
#[derivative(Debug)]
pub struct AddressIndex(
    #[derivative(Debug(bound = "", format_with = "crate::fmt_hex"))] pub [u8; 11],
);

impl From<u8> for AddressIndex {
    fn from(x: u8) -> Self {
        let mut bytes = [0; 11];
        bytes[0] = x;
        Self(bytes)
    }
}

impl From<u16> for AddressIndex {
    fn from(x: u16) -> Self {
        let mut bytes = [0; 11];
        bytes[0..2].copy_from_slice(&x.to_le_bytes());
        Self(bytes)
    }
}

impl From<u32> for AddressIndex {
    fn from(x: u32) -> Self {
        let mut bytes = [0; 11];
        bytes[0..4].copy_from_slice(&x.to_le_bytes());
        Self(bytes)
    }
}

impl From<u64> for AddressIndex {
    fn from(x: u64) -> Self {
        let mut bytes = [0; 11];
        bytes[0..8].copy_from_slice(&x.to_le_bytes());
        Self(bytes)
    }
}

impl From<usize> for AddressIndex {
    fn from(x: usize) -> Self {
        (x as u64).into()
    }
}

impl From<AddressIndex> for u128 {
    fn from(x: AddressIndex) -> Self {
        let mut bytes = [0; 16];
        bytes[0..11].copy_from_slice(&x.0);
        u128::from_le_bytes(bytes)
    }
}

impl TryFrom<AddressIndex> for u64 {
    type Error = anyhow::Error;
    fn try_from(address_index: AddressIndex) -> Result<Self, Self::Error> {
        let bytes = &address_index.0;
        if bytes[8] == 0 && bytes[9] == 0 && bytes[10] == 0 {
            Ok(u64::from_le_bytes(
                bytes[0..8]
                    .try_into()
                    .expect("can take first 8 bytes of 11-byte array"),
            ))
        } else {
            Err(anyhow::anyhow!("address index out of range"))
        }
    }
}

impl TryFrom<&[u8]> for AddressIndex {
    type Error = anyhow::Error;

    fn try_from(slice: &[u8]) -> Result<AddressIndex, Self::Error> {
        if slice.len() != DIVERSIFIER_LEN_BYTES {
            return Err(anyhow!(
                "address index must be 11 bytes, got {:?}",
                slice.len()
            ));
        }

        let mut bytes = [0u8; DIVERSIFIER_LEN_BYTES];
        bytes.copy_from_slice(&slice[0..11]);
        Ok(AddressIndex(bytes))
    }
}

impl Protobuf<pb::AddressIndex> for AddressIndex {}

impl From<AddressIndex> for pb::AddressIndex {
    fn from(d: AddressIndex) -> pb::AddressIndex {
        pb::AddressIndex {
            inner: d.0.to_vec(),
        }
    }
}

impl TryFrom<pb::AddressIndex> for AddressIndex {
    type Error = anyhow::Error;

    fn try_from(d: pb::AddressIndex) -> Result<AddressIndex, Self::Error> {
        Ok(AddressIndex(d.inner.as_slice().try_into()?))
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn address_index_strategy() -> BoxedStrategy<AddressIndex> {
        any::<[u8; 11]>().prop_map(AddressIndex).boxed()
    }

    fn diversifier_key_strategy() -> BoxedStrategy<DiversifierKey> {
        any::<[u8; 32]>().prop_map(DiversifierKey).boxed()
    }

    proptest! {
        #[test]
        fn diversifier_encryption_roundtrip(
            key in diversifier_key_strategy(),
            index in address_index_strategy(),
        ) {
            let diversifier = key.diversifier_for_index(&index);
            let index2 = key.index_for_diversifier(&diversifier);
            assert_eq!(index2, index );
        }
    }
}
