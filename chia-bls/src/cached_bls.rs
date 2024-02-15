extern crate lru;
use lru::LruCache;
use std::num::NonZeroUsize;
use crate::Signature;
use crate::hash_to_g2;
use crate::aggregate_verify as agg_ver;
use crate::gtelement::GTElement;
use crate::PublicKey;
use std::collections::HashMap;
use sha2::{Digest, Sha256};

pub type Bytes32 = [u8; 32];
pub type Bytes48 = [u8; 48];

pub struct BLSCache {
    cache: LruCache<Bytes32, GTElement>,
}

impl BLSCache {
    
    pub fn generator(cache_size: Option<usize>) -> Self {
        let cache: LruCache<Bytes32, GTElement> = LruCache::new(NonZeroUsize::new(cache_size.unwrap_or(50000)).unwrap());
        Self{cache}
    }
    
    // Define a function to get pairings
    pub fn get_pairings(
        &mut self,
        pks: &[Bytes48],
        msgs: &[Vec<u8>],
        force_cache: bool,
    ) -> Vec<GTElement> {
        let mut pairings: Vec<Option<GTElement>> = vec![];
        let mut missing_count: usize = 0;
        
        for (pk, msg) in pks.iter().zip(msgs.iter()) {
            let mut aug_msg = pk.to_vec();
            aug_msg.extend_from_slice(msg); // pk + msg
            let mut hasher = Sha256::new();
            hasher.update(aug_msg);
            let h: Bytes32 = hasher.finalize().into();
            let pairing: Option<&GTElement> = self.cache.get(&h);
            match pairing {
                Some(pairing) => {
                    if !force_cache {
                        // Heuristic to avoid more expensive sig validation with pairing
                        // cache when it's empty and cached pairings won't be useful later
                        // (e.g. while syncing)
                        missing_count += 1;
                        if missing_count > pks.len() / 2 {
                            return vec![];
                        }
                    }
                    pairings.push(Some(pairing.clone()));
                },
                _ => {
                    pairings.push(None);
                },
            }
            
        }

        // G1Element.from_bytes can be expensive due to subgroup check, so we avoid recomputing it with this cache
        let mut pk_bytes_to_g1: HashMap<Bytes48, PublicKey> = HashMap::new();
        let mut ret: Vec<GTElement> = vec![];

        for (i, pairing) in pairings.iter_mut().enumerate() {
            if let Some(pairing) = pairing {  // equivalent to `if pairing is not None`
                ret.push(pairing.clone());
            } else {
                let mut aug_msg = pks[i].to_vec();
                aug_msg.extend_from_slice(&msgs[i]);  // pk + msg
                let aug_hash = hash_to_g2(&aug_msg);

                let pk_parsed = pk_bytes_to_g1.entry(pks[i]).or_insert_with(|| {
                    PublicKey::from_bytes(&pks[i]).unwrap()
                });

                let pairing = aug_hash.pair(pk_parsed);
                let mut hasher = Sha256::new();
                hasher.update(&aug_msg);
                let h: Bytes32 = hasher.finalize().into();
                self.cache.put(h, pairing.clone());
                ret.push(pairing);
            }
        }

        ret
    }

    pub fn aggregate_verify(
        &mut self,
        pks: &Vec<Bytes48>,
        msgs: &Vec<Vec<u8>>,
        sig: &Signature,
        force_cache: bool, 
    ) -> bool {
        let mut pairings: Vec<GTElement> = self.get_pairings(&pks, &msgs, force_cache);
        if pairings.is_empty() {
            let mut data = Vec::<(PublicKey, Vec<u8>)>::new();
            for (pk, msg) in pks.iter().zip(msgs.iter()) {
                let pk = PublicKey::from_bytes_unchecked(pk).unwrap();
                data.push((pk.clone(), msg.clone()));
            }
            let res: bool = agg_ver(sig, data);
            return res
        }
        let pairings_prod = pairings.pop(); // start with the first pairing
        match pairings_prod {
            Some(mut prod) => {
                for p in pairings.iter() {  // loop through rest of list
                    prod *= &p;
                }
                prod == sig.pair(&PublicKey::generator())
            },
            _ => {
                pairings.len() == 0
            },
        }
        
    }
}

#[cfg(test)]
pub mod tests {
    use crate::SecretKey;
    use crate::sign;
    use super::*;

    #[test]
    pub fn test_instantiation() {
        let mut bls_cache: BLSCache = BLSCache::generator(None);
        let byte_array: [u8; 32] = [0; 32];
        let sk: SecretKey = SecretKey::from_seed(&byte_array);
        let pk:PublicKey = sk.public_key();
        let msg: [u8; 32] = [106; 32];
        let mut aug_msg: Vec<u8> = pk.clone().to_bytes().to_vec();
        aug_msg.extend_from_slice(&msg);  // pk + msg
        let aug_hash = hash_to_g2(&aug_msg);
        let pairing = aug_hash.pair(&pk);
        let mut hasher = Sha256::new();
        hasher.update(&aug_msg);
        let h: Bytes32 = hasher.finalize().into();
        bls_cache.cache.put(h, pairing.clone());
        assert_eq!(*bls_cache.cache.get(&h).unwrap(), pairing);
    }

    #[test]
    pub fn test_aggregate_verify() {
        let mut bls_cache: BLSCache = BLSCache::generator(None);
        assert_eq!(bls_cache.cache.len(), 0);
        let byte_array: [u8; 32] = [0; 32];
        let sk: SecretKey = SecretKey::from_seed(&byte_array);
        let pk: PublicKey = sk.public_key();
        let msg: Vec<u8> = [106; 32].to_vec();
        let sig: Signature = sign(&sk, &msg);
        let pk_list: Vec<[u8; 48]> = [pk.to_bytes()].to_vec();
        let msg_list: Vec<Vec<u8>> = [msg].to_vec();
        assert!(bls_cache.aggregate_verify(&pk_list, &msg_list, &sig, true));
        assert_eq!(bls_cache.cache.len(), 1);
    }
}

