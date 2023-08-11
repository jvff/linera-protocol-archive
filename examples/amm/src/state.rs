use linera_sdk::{
    base::Amount,
    views::{RegisterView, ViewStorageContext},
};
use linera_views::views::{GraphQLView, RootView};

#[derive(RootView, GraphQLView)]
#[view(context = "ViewStorageContext")]
pub struct Amm {
    pub total_shares: RegisterView<Amount>,
}
