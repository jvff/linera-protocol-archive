mod exported_future;

use async_trait::async_trait;
use std::error::Error;

pub use crate::exported_future::ExportedFuture;

#[async_trait]
pub trait Application {
    /// Message reports for application execution errors.
    type Error: Error;

    /// Apply an operation from the current block.
    async fn apply_operation(
        &mut self,
        context: &OperationContext,
        operation: &[u8],
    ) -> Result<ApplicationResult, Self::Error>;

    /// Apply an effect originating from a cross-chain message.
    async fn apply_effect(
        &mut self,
        context: &EffectContext,
        effect: &[u8],
    ) -> Result<ApplicationResult, Self::Error>;

    /// Allow an operation or an effect of other applications to call into this
    /// application.
    async fn call(
        &mut self,
        context: &CalleeContext,
        name: &str,
        argument: &[u8],
    ) -> Result<(Vec<u8>, ApplicationResult), Self::Error>;

    /// Allow an end user to execute read-only queries on the state of this application.
    /// NOTE: This is not meant to be metered and may not be exposed by validators.
    async fn query(
        &self,
        context: &QueryContext,
        name: &str,
        argument: &[u8],
    ) -> Result<Vec<u8>, Self::Error>;
}

#[derive(Debug, Clone)]
pub struct OperationContext {
    pub chain_id: ChainId,
    pub height: BlockHeight,
    pub index: u64,
}

#[derive(Debug, Clone)]
pub struct EffectContext {
    pub chain_id: ChainId,
    pub height: BlockHeight,
    pub effect_id: EffectId,
}

#[derive(Debug, Clone)]
pub struct CalleeContext {
    pub chain_id: ChainId,
    /// `None` if the caller doesn't want this particular call to be authenticated (e.g.
    /// for safety reasons).
    pub authenticated_caller_id: Option<ApplicationId>,
}

#[derive(Debug, Clone)]
pub struct QueryContext {
    pub chain_id: ChainId,
}

#[derive(Debug)]
pub struct ApplicationResult {
    pub effects: Vec<(Destination, Vec<u8>)>,
    pub subscribe: Vec<(String, ChainId)>,
    pub unsubscribe: Vec<(String, ChainId)>,
}

/// The index of an effect in a chain.
#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Hash, Debug)]
pub struct EffectId {
    pub chain_id: ChainId,
    pub height: BlockHeight,
    pub index: u64,
}

/// The unique identifier (UID) of a chain. This is the hash value of a ChainDescription.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Hash)]
pub struct ChainId(pub HashValue);

impl ChainId {
    pub fn from_bytes_unchecked(bytes: &[u8]) -> Self {
        let hash_bytes = bytes
            .try_into()
            .expect("Host provided invalid Chain ID bytes");
        let hash = HashValue(hash_bytes);
        ChainId(hash)
    }

    pub fn to_bytes(&self) -> &[u8] {
        let hash_value = &self.0;
        &hash_value.0
    }
}

/// A block height to identify blocks in a chain.
#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Hash, Default, Debug)]
pub struct BlockHeight(pub u64);

/// A Sha512 value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct HashValue([u8; 64]);

/// The destination of a message, relative to a particular application.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Destination {
    /// Direct message to a chain.
    Recipient(ChainId),
    /// Broadcast to the current subscribers of our channel.
    Subscribers(String),
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Hash, Default, Debug)]
pub struct ApplicationId(pub u64);
