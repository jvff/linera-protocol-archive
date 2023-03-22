#![cfg(not(target_arch = "wasm32"))]

use fungible::{test_utils::InitialStateBuilder, Account, AccountOwner, Operation};
use linera_sdk::test::{TestValidator, ToBcsBytes};

#[tokio::test]
async fn cross_chain_transfer() {
    let initial_amount = 20_u64;
    let transfer_amount = 15_u64;

    let (validator, bytecode_id) = TestValidator::with_current_bytecode().await;
    let mut sender_chain = validator.new_chain().await;
    let sender_account = AccountOwner::from(sender_chain.public_key());

    let initial_state = InitialStateBuilder::default().with_account(sender_account, initial_amount);
    let application_id = sender_chain
        .create_application(bytecode_id, vec![], initial_state.build(), vec![])
        .await;

    let receiver_chain = validator.new_chain().await;
    let receiver_account = AccountOwner::from(receiver_chain.public_key());

    sender_chain
        .add_block(|block| {
            block.with_operation(
                application_id,
                Operation::Transfer {
                    owner: sender_account,
                    amount: transfer_amount.into(),
                    target_account: Account {
                        chain_id: receiver_chain.id(),
                        owner: receiver_account,
                    },
                }
                .to_bcs_bytes(),
            );
        })
        .await;

    assert_eq!(
        sender_chain
            .query(application_id, sender_account.to_bcs_bytes())
            .await,
        (initial_amount - transfer_amount).to_bcs_bytes()
    );

    receiver_chain.handle_received_effects().await;

    assert_eq!(
        receiver_chain
            .query(application_id, receiver_account.to_bcs_bytes())
            .await,
        transfer_amount.to_bcs_bytes()
    );
}
