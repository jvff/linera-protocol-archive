use super::ChainGuards;
use futures::FutureExt;
use linera_base::messages::ChainId;
use sha2::{Digest, Sha512};
use std::time::Duration;
use tokio::time::sleep;

/// Test if a chain guard can be obtained again after it has been dropped.
#[tokio::test(start_paused = true)]
async fn guard_can_be_obtained_later_again() {
    let chain_id = create_dummy_chain_id('0');
    let mut guards = ChainGuards::default();
    // Obtain the guard the first time and drop it immediately
    let _ = guards.guard(chain_id).await;
    // Wait before obtaining the guard again
    sleep(Duration::from_secs(10)).await;
    // It should be available immediately on the second time
    assert!(guards.guard(chain_id).now_or_never().is_some());
}

/// Create a dummy [`ChainId`] by repeating the provided nibble.
pub fn create_dummy_chain_id(nibble: char) -> ChainId {
    assert!(('0'..='9').contains(&nibble) || ('a'..='f').contains(&nibble));

    nibble
        .to_string()
        .repeat(Sha512::output_size() * 2)
        .parse()
        .expect("Invalid chain ID")
}
