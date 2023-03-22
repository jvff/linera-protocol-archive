use crate::{ApplicationId, BlockHeight, BytecodeId, ChainId, CryptoHash, EffectId};
use linera_base::{crypto, data_types as base};
use linera_execution as execution;

impl From<base::ChainDescription> for ChainId {
    fn from(chain_description: base::ChainDescription) -> Self {
        base::ChainId::from(chain_description).into()
    }
}

impl From<base::ChainId> for ChainId {
    fn from(chain_id: base::ChainId) -> Self {
        ChainId(chain_id.0.into())
    }
}

impl From<ChainId> for base::ChainId {
    fn from(chain_id: ChainId) -> Self {
        base::ChainId(chain_id.0.into())
    }
}

impl From<BytecodeId> for execution::BytecodeId {
    fn from(bytecode_id: BytecodeId) -> Self {
        execution::BytecodeId(bytecode_id.0.into())
    }
}

impl From<ApplicationId> for execution::ApplicationId {
    fn from(application_id: ApplicationId) -> Self {
        execution::ApplicationId::User(application_id.into())
    }
}

impl From<ApplicationId> for execution::UserApplicationId {
    fn from(application_id: ApplicationId) -> Self {
        execution::UserApplicationId {
            bytecode_id: application_id.bytecode.into(),
            creation: application_id.creation.into(),
        }
    }
}

impl From<EffectId> for base::EffectId {
    fn from(effect_id: EffectId) -> Self {
        base::EffectId {
            chain_id: effect_id.chain_id.into(),
            height: effect_id.height.into(),
            index: effect_id
                .index
                .try_into()
                .expect("Incompatible `EffectId` types in `linera-sdk` and `linera-base`"),
        }
    }
}

impl From<crypto::CryptoHash> for CryptoHash {
    fn from(crypto_hash: crypto::CryptoHash) -> Self {
        CryptoHash(
            crypto_hash
                .as_bytes()
                .as_slice()
                .try_into()
                .expect("Incompatible `CryptoHash` types in `linera-sdk` and `linera-base`"),
        )
    }
}

impl From<CryptoHash> for crypto::CryptoHash {
    fn from(crypto_hash: CryptoHash) -> Self {
        crypto::CryptoHash::try_from(&crypto_hash.0[..])
            .expect("Incompatible `CryptoHash` types in `linera-base` and `linera-sdk`")
    }
}

impl From<base::BlockHeight> for BlockHeight {
    fn from(block_height: base::BlockHeight) -> Self {
        BlockHeight(block_height.0)
    }
}

impl From<BlockHeight> for base::BlockHeight {
    fn from(block_height: BlockHeight) -> Self {
        base::BlockHeight(block_height.0)
    }
}
