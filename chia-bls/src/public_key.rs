use blst::{
    blst_core_verify_pk_in_g1, blst_p1 as P1, blst_p1_add_affine, blst_p1_affine as P1Affine,
    blst_p1_affine_compress, blst_p1_affine_in_g1, blst_p1_affine_is_inf, blst_p1_from_affine,
    blst_p1_to_affine, blst_p1_uncompress, BLST_ERROR,
};
use sha2::{digest::FixedOutput, Digest, Sha256};

use crate::{
    aug_scheme::{prepend_message, DST},
    DerivableKey, SecretKey, Signature,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey(pub(crate) P1Affine);

impl PublicKey {
    pub fn validate(bytes: &[u8; 48]) -> Option<Self> {
        let result = Self::from_bytes(bytes)?;

        if result.is_valid() {
            Some(result)
        } else {
            None
        }
    }

    pub fn from_bytes(bytes: &[u8; 48]) -> Option<Self> {
        if (bytes[0] & 0x80) != 0 {
            let mut p1_affine = P1Affine::default();
            if unsafe { blst_p1_uncompress(&mut p1_affine, bytes.as_ptr()) }
                == BLST_ERROR::BLST_SUCCESS
            {
                Some(Self(p1_affine))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn to_bytes(&self) -> [u8; 48] {
        let mut bytes = [0u8; 48];
        unsafe {
            blst_p1_affine_compress(bytes.as_mut_ptr(), &self.0);
        }
        bytes
    }

    pub fn is_valid(&self) -> bool {
        unsafe { !blst_p1_affine_is_inf(&self.0) && blst_p1_affine_in_g1(&self.0) }
    }

    pub fn add(&self, public_key: &Self) -> Self {
        let mut p1 = P1::default();
        unsafe {
            blst_p1_from_affine(&mut p1, &self.0);
        }

        let mut output_p1 = P1::default();
        unsafe {
            blst_p1_add_affine(&mut output_p1, &p1, &public_key.0);
        }

        let mut p1_affine = P1Affine::default();
        unsafe {
            blst_p1_to_affine(&mut p1_affine, &output_p1);
        }
        Self(p1_affine)
    }

    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        if !self.is_valid() || !signature.is_valid(false) {
            return false;
        }

        let message = prepend_message(self, message);

        unsafe {
            blst_core_verify_pk_in_g1(
                &self.0,
                &signature.0,
                true,
                message.as_ptr(),
                message.len(),
                DST.as_ptr(),
                DST.len(),
                [].as_ptr(),
                0,
            ) == BLST_ERROR::BLST_SUCCESS
        }
    }
}

impl DerivableKey for PublicKey {
    fn derive_unhardened(&self, index: u32) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(self.to_bytes());
        hasher.update(index.to_be_bytes());
        let digest: [u8; 32] = hasher.finalize_fixed().into();

        let new_sk = SecretKey::from_bytes(&digest).unwrap();
        self.add(&new_sk.to_public_key())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::secret_key::SecretKey;

    use hex::FromHex;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    #[test]
    fn test_derive_unhardened() {
        let sk_hex = "52d75c4707e39595b27314547f9723e5530c01198af3fc5849d9a7af65631efb";
        let sk = SecretKey::from_bytes(&<[u8; 32]>::from_hex(sk_hex).unwrap()).unwrap();
        let pk = sk.to_public_key();

        // make sure deriving the secret keys produce the same public keys as
        // deriving the public key
        for idx in 0..4_usize {
            let derived_sk = sk.derive_unhardened(idx as u32);
            let derived_pk = pk.derive_unhardened(idx as u32);
            assert_eq!(derived_pk.to_bytes(), derived_sk.to_public_key().to_bytes());
        }
    }

    #[test]
    fn test_from_bytes() {
        let mut rng = StdRng::seed_from_u64(1337);
        let mut data = [0u8; 48];
        for _i in 0..50 {
            rng.fill(data.as_mut_slice());
            // just any random bytes are not a valid key and should fail
            assert_eq!(PublicKey::validate(&data), None);
        }
    }

    #[test]
    fn test_roundtrip() {
        let mut rng = StdRng::seed_from_u64(1337);
        let mut data = [0u8; 32];
        for _i in 0..50 {
            rng.fill(data.as_mut_slice());
            let sk = SecretKey::from_seed(&data);
            let pk = sk.to_public_key();
            let bytes = pk.to_bytes();
            let pk2 = PublicKey::from_bytes(&bytes).unwrap();
            assert_eq!(pk, pk2);
        }
    }
}
