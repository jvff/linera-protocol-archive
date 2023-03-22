#![cfg(not(target_arch = "wasm32"))]

use fungible::{
    test_utils::InitialStateBuilder, AccountOwner, Nonce, SignedTransferPayload, Transfer,
};
use linera_sdk::{
    crypto::KeyPair,
    test::{TestValidator, ToBcsBytes},
};

#[tokio::test]
async fn cross_chain_transfer() {
    let initial_amount = 20_u128;
    let transfer_amount = 15_u128;

    let mut initial_state = InitialStateBuilder::default();
    let sender_keys = initial_state.add_account(initial_amount);
    let receiver_keys = KeyPair::generate();

    let (validator, application_id) =
        TestValidator::with_current_application(vec![], initial_state.build()).await;
    let sender_chain = validator.get_chain(&application_id.creation.chain_id);
    let receiver_chain = validator.new_chain().await;

    let sender_chain_id = sender_chain.id();
    sender_chain
        .add_block(|block| {
            block.with_operation(
                application_id,
                SignedTransferPayload {
                    token_id: application_id,
                    source_chain: sender_chain_id,
                    nonce: Nonce::default(),
                    transfer: Transfer {
                        destination_account: receiver_keys.public().into(),
                        destination_chain: receiver_chain.id(),
                        amount: transfer_amount,
                    },
                }
                .sign(&sender_keys)
                .to_bcs_bytes(),
            );
        })
        .await;

    assert_eq!(
        sender_chain
            .query(
                application_id,
                AccountOwner::from(sender_keys.public()).to_bcs_bytes()
            )
            .await,
        (initial_amount - transfer_amount).to_bcs_bytes()
    );

    receiver_chain.handle_received_effects().await;

    assert_eq!(
        receiver_chain
            .query(
                application_id,
                AccountOwner::from(receiver_keys.public()).to_bcs_bytes()
            )
            .await,
        transfer_amount.to_bcs_bytes()
    );
}
