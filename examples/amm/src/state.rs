use linera_sdk::views::{MapView, ViewStorageContext};
use linera_views::views::{GraphQLView, RootView};

#[derive(RootView, GraphQLView)]
#[view(context = "ViewStorageContext")]
pub struct Amm {
    pub token_pool: MapView<String, u64>,
}
