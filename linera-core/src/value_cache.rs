// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! A least-recently used cache of [`HashedCertificateValue`]s.

use std::{borrow::Cow, collections::BTreeSet, num::NonZeroUsize};

use linera_base::crypto::CryptoHash;
use linera_chain::data_types::{Certificate, HashedCertificateValue, LiteCertificate};
use lru::LruCache;
use tokio::sync::Mutex;

use crate::worker::WorkerError;

/// The default cache size.
const DEFAULT_VALUE_CACHE_SIZE: usize = 1000;

/// A least-recently used cache of [`HashedCertificateValue`]s.
pub struct CertificateValueCache {
    cache: Mutex<LruCache<CryptoHash, HashedCertificateValue>>,
}

impl Default for CertificateValueCache {
    fn default() -> Self {
        let size = NonZeroUsize::try_from(DEFAULT_VALUE_CACHE_SIZE)
            .expect("Default cache size is larger than zero");

        CertificateValueCache {
            cache: Mutex::new(LruCache::new(size)),
        }
    }
}

impl CertificateValueCache {
    /// Returns a [`BTreeSet`] of the hashes in the cache.
    pub async fn keys(&self) -> BTreeSet<CryptoHash> {
        self.cache
            .lock()
            .await
            .iter()
            .map(|(key, _)| *key)
            .collect()
    }

    /// Returns [`true`] if the cache contains the [`HashedCertificateValue`] with the
    /// requested [`CryptoHash`].
    pub async fn contains(&self, hash: &CryptoHash) -> bool {
        self.cache.lock().await.contains(hash)
    }

    /// Returns a [`HashedCertificateValue`] from the cache, if present.
    pub async fn get(&self, hash: &CryptoHash) -> Option<HashedCertificateValue> {
        self.cache.lock().await.get(hash).cloned()
    }

    /// Populates a [`LiteCertificate`] with its [`CertificateValue`], if it's present in
    /// the cache.
    pub async fn full_certificate(
        &self,
        certificate: LiteCertificate<'_>,
    ) -> Result<Certificate, WorkerError> {
        let value = self
            .get(&certificate.value.value_hash)
            .await
            .ok_or(WorkerError::MissingCertificateValue)?;
        certificate
            .with_value(value)
            .ok_or(WorkerError::InvalidLiteCertificate)
    }

    /// Inserts a [`HashedCertificateValue`] into the cache, if it's not already present.
    ///
    /// The `value` is wrapped in a [`Cow`] so that it is only cloned if it needs to be
    /// inserted in the cache.
    ///
    /// Returns [`true`] if the value was not already present in the cache.
    pub async fn insert<'a>(&self, value: Cow<'a, HashedCertificateValue>) -> bool {
        let hash = value.hash();
        let mut cache = self.cache.lock().await;
        if cache.contains(&hash) {
            return false;
        }
        // Cache the certificate so that clients don't have to send the value again.
        cache.push(hash, value.into_owned());
        true
    }

    /// Inserts the validated block and the corresponding confirmed block.
    pub async fn insert_validated_and_confirmed(&self, value: &HashedCertificateValue) {
        if self.insert(Cow::Borrowed(value)).await {
            if let Some(value) = value.validated_to_confirmed() {
                self.insert(Cow::Owned(value)).await;
            }
        }
    }
}
