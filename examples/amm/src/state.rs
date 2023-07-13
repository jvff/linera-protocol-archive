use linera_sdk::views::{RegisterView, ViewStorageContext};
use linera_views::views::{GraphQLView, RootView};

#[derive(RootView, GraphQLView)]
#[view(context = "ViewStorageContext")]
pub struct Amm {
    pub balance0: RegisterView<u64>,
    pub balance1: RegisterView<u64>,
    pub total_shares: RegisterView<u64>,
}
