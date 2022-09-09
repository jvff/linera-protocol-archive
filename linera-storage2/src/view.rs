use crate::chain::{ChainStateView, InnerChainStateView, InnerChainStateViewContext};
use linera_base::messages::ChainId;
use linera_views::{
    impl_view,
    views::{CollectionOperations, ScopedView, SharedCollectionView},
};

/// A view accessing the validator's storage.
#[derive(Debug)]
pub struct StorageView<C> {
    pub chain_states: ScopedView<0, SharedCollectionView<C, ChainId, InnerChainStateView<C>>>,
}

impl_view! {
    StorageView {
        chain_states,
    };
    CollectionOperations<ChainId>,
    InnerChainStateViewContext,
}

impl<C> StorageView<C>
where
    C: StorageViewContext,
{
    pub async fn load_chain(&mut self, id: ChainId) -> Result<ChainStateView<C>, C::Error> {
        Ok(self.chain_states.load_entry(id).await?.into())
    }
}
