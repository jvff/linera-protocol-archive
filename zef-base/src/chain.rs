// Copyright (c) Facebook, Inc. and its affiliates.
// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    base_types::*, committee::Committee, ensure, error::Error, manager::ChainManager, messages::*,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// State of a chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct ChainState {
    /// The UID of the chain.
    pub id: ChainId,
    /// How the chain was created. May be unknown for inactive chains.
    pub description: Option<ChainDescription>,
    /// Execution state.
    pub state: ExecutionState,
    /// Hash of the execution state.
    pub state_hash: HashValue,
    /// Hash of the latest certified block in this chain, if any.
    pub block_hash: Option<HashValue>,
    /// Sequence number tracking blocks.
    pub next_block_height: BlockHeight,

    /// Hashes of all certified blocks for this sender.
    /// This ends with `block_hash` and has length `usize::from(next_block_height)`.
    pub confirmed_log: Vec<HashValue>,
    /// Hashes of all certified blocks known as a receiver (local ordering).
    pub received_log: Vec<HashValue>,

    /// Mailboxes used to send messages, indexed by recipient.
    pub outboxes: HashMap<ChainId, OutboxState>,
    /// Mailboxes used to receive messages, indexed by sender.
    pub inboxes: HashMap<ChainId, InboxState>,
}

/// An outbox used to send messages to another chain. NOTE: Messages are implied by the
/// execution of blocks, so currently we just send the certified blocks over and let the
/// receivers figure out what was the message for them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct OutboxState {
    /// Keep sending these certified blocks of ours until they are acknowledged by
    /// receivers. Keep the height around so that we can quickly dequeue.
    pub queue: VecDeque<(BlockHeight, HashValue)>,
}

/// An inbox used to receive and execute messages from another chain.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct InboxState {
    /// We have already received the cross-chain requests and enqueued all the messages
    /// below this height.
    pub next_height_to_receive: BlockHeight,
    /// These events have been received but not yet picked by a block to be executed.
    pub received_events: VecDeque<Event>,
    /// These events have been executed but the cross-chain requests have not been
    /// received yet.
    pub expected_events: VecDeque<Event>,
}

/// A message sent by some (unspecified) chain at a particular height and index.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct Event {
    /// The height of the block that created the event.
    pub height: BlockHeight,
    /// The index of the operation.
    pub index: usize,
    /// The operation that created the event.
    pub operation: Operation,
}

/// Execution state of a chain.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct ExecutionState {
    /// Whether our reconfigurations are managed by a "beacon" chain, or if we are it and
    /// managing other chains.
    pub status: Option<ChainStatus>,
    /// Current committee, if any.
    pub committee: Option<Committee>,
    /// Manager of the chain.
    pub manager: ChainManager,
    /// Balance of the chain.
    pub balance: Balance,
}

impl BcsSignable for ExecutionState {}

/// The administrative status of this chain w.r.t reconfigurations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum ChainStatus {
    ManagedBy { admin_id: ChainId },
    Managing { subscribers: Vec<ChainId> },
}

impl ChainState {
    pub fn new(id: ChainId) -> Self {
        let state = ExecutionState::default();
        let state_hash = HashValue::new(&state);
        Self {
            id,
            description: None,
            state,
            state_hash,
            block_hash: None,
            next_block_height: BlockHeight::default(),
            confirmed_log: Vec::new(),
            received_log: Vec::new(),
            inboxes: HashMap::new(),
            outboxes: HashMap::new(),
        }
    }

    pub fn create(
        committee: Committee,
        admin_id: ChainId,
        description: ChainDescription,
        owner: Owner,
        balance: Balance,
    ) -> Self {
        let status = if ChainId::from(description) == admin_id {
            ChainStatus::Managing {
                subscribers: Vec::new(),
            }
        } else {
            ChainStatus::ManagedBy { admin_id }
        };
        let mut chain = Self::new(description.into());
        chain.description = Some(description);
        chain.state.status = Some(status);
        chain.state.committee = Some(committee);
        chain.state.manager = ChainManager::single(owner);
        chain.state.balance = balance;
        chain.state_hash = HashValue::new(&chain.state);
        chain
    }

    pub fn is_active(&self) -> bool {
        self.description.is_some()
            && self.state.manager.is_active()
            && self.state.committee.is_some()
            && self.state.status.is_some()
    }

    pub fn make_chain_info(&self, key_pair: Option<&KeyPair>) -> ChainInfoResponse {
        let info = ChainInfo {
            chain_id: self.id,
            description: self.description,
            manager: self.state.manager.clone(),
            balance: self.state.balance,
            admin_id: self.state.admin_id(self.id),
            block_hash: self.block_hash,
            next_block_height: self.next_block_height,
            state_hash: self.state_hash,
            queried_committee: None,
            queried_pending_messages: Vec::new(),
            queried_sent_certificates: Vec::new(),
            count_received_certificates: self.received_log.len(),
            queried_received_certificates: Vec::new(),
        };
        ChainInfoResponse::new(info, key_pair)
    }

    /// Verify that this chain is up-to-date and all the messages executed ahead of time
    /// have been properly received by now.
    pub fn validate_incoming_messages(&self) -> Result<(), Error> {
        for (&sender_id, inbox) in &self.inboxes {
            ensure!(
                inbox.expected_events.is_empty(),
                Error::MissingCrossChainUpdate {
                    sender_id,
                    height: inbox.expected_events.front().unwrap().height,
                }
            );
        }
        Ok(())
    }

    /// Whether an invalid operation for this block can become valid later.
    pub fn is_retriable_validation_error(error: &Error) -> bool {
        matches!(error, Error::MissingCrossChainUpdate { .. })
    }

    /// Schedule operations to be executed as a recipient, unless this block was already
    /// processed. Returns true if the call changed the chain state. Operations must be
    /// received by order of heights and indices.
    pub fn receive_block(
        &mut self,
        sender_id: ChainId,
        height: BlockHeight,
        operations: Vec<Operation>,
        key: HashValue,
    ) -> Result<bool, Error> {
        let inbox = self.inboxes.entry(sender_id).or_default();
        if height < inbox.next_height_to_receive {
            // We already received this block.
            return Ok(false);
        }
        // Mark the block as received.
        inbox.next_height_to_receive = height.try_add_one()?;
        self.received_log.push(key);

        let mut has_processed_operations = false;
        for (index, operation) in operations.into_iter().enumerate() {
            if !self.state.has_recipient(&operation, self.id) {
                continue;
            }
            // Chain creation is a special operation that can only be executed (once) in this callback.
            if let Operation::OpenChain {
                id,
                owner,
                committee,
                admin_id,
                ..
            } = &operation
            {
                if id == &self.id {
                    // guaranteed under BFT assumptions.
                    assert!(self.description.is_none());
                    assert!(!self.state.manager.is_active());
                    assert!(self.state.committee.is_none());
                    let description = ChainDescription::Child(OperationId {
                        chain_id: sender_id,
                        height,
                        index,
                    });
                    assert_eq!(self.id, description.into());
                    self.description = Some(description);
                    self.state.committee = Some(committee.clone());
                    self.state.status = Some(ChainStatus::ManagedBy {
                        admin_id: *admin_id,
                    });
                    self.state.manager = ChainManager::single(*owner);
                    self.state_hash = HashValue::new(&self.state);
                    // Proceed to scheduling the `OpenChain` operation for "execution".
                    // Although it won't do anything, it's simpler than asking block producers
                    // to skip this kind of incoming messages.
                }
            }
            // Find if the message was executed ahead of time.
            if let Some(event) = inbox.expected_events.front() {
                if height == event.height && index == event.index && operation == event.operation {
                    // We already executed this message. Remove it from the queue.
                    inbox.expected_events.pop_front();
                    return Ok(true);
                }
                // Should be unreachable under BFT assumptions.
                panic!("Given the confirmed blocks that we have seen, we were expecting a different message to come first.");
            }
            // Otherwise, schedule the message for execution.
            inbox.received_events.push_back(Event {
                height,
                index,
                operation,
            });
            has_processed_operations = true;
        }
        // Update the state hash at the end.
        ensure!(has_processed_operations, Error::InvalidCrossChainRequest);
        Ok(true)
    }

    /// Execute a new block: first the incoming messages, then the main operation. In case
    /// of errors, `self` may not be consistent any more and should be thrown away.
    pub fn execute_block(&mut self, block: &Block) -> Result<HashSet<ChainId>, Error> {
        // First, process incoming messages.
        for message_group in &block.incoming_messages {
            for (message_index, message_operation) in &message_group.operations {
                // Reconcile the operation with the received queue, or mark it as "expected".
                let inbox = self.inboxes.entry(message_group.sender_id).or_default();
                match inbox.received_events.front() {
                    Some(Event {
                        height,
                        index,
                        operation,
                    }) => {
                        ensure!(
                            message_group.height == *height && message_index == index,
                            Error::InvalidMessageOrder {
                                sender_id: message_group.sender_id,
                                height: message_group.height,
                                index: *message_index,
                                expected_height: *height,
                                expected_index: *index,
                            }
                        );
                        ensure!(
                            message_operation == operation,
                            Error::InvalidMessageContent {
                                sender_id: message_group.sender_id,
                                height: message_group.height,
                                index: *message_index,
                            }
                        );
                        inbox.received_events.pop_front().unwrap();
                    }
                    None => {
                        inbox.expected_events.push_back(Event {
                            height: message_group.height,
                            index: *message_index,
                            operation: message_operation.clone(),
                        });
                    }
                }
                // Execute the received operation.
                self.state
                    .apply_operation_as_recipient(self.id, message_operation)?;
            }
        }
        // Second, execute the operations in the block and remember recipients to notify.
        let mut recipients = HashSet::new();
        for (index, operation) in block.operations.iter().enumerate() {
            recipients.extend(self.state.apply_operation_as_sender(
                block.chain_id,
                block.height,
                index,
                operation,
            )?);
        }
        // Last, recompute the state hash.
        self.state_hash = HashValue::new(&self.state);
        Ok(recipients)
    }
}

impl ExecutionState {
    pub fn admin_id(&self, chain_id: ChainId) -> Option<ChainId> {
        match self.status.as_ref()? {
            ChainStatus::ManagedBy { admin_id } => Some(*admin_id),
            ChainStatus::Managing { .. } => Some(chain_id),
        }
    }

    fn has_recipient(&self, operation: &Operation, chain_id: ChainId) -> bool {
        use Operation::*;
        match operation {
            Transfer {
                recipient: Address::Account(id),
                ..
            } => chain_id == *id,
            OpenChain { id, admin_id, .. } => chain_id == *id || chain_id == *admin_id,
            _ => false,
        }
    }

    /// Execute the sender's side of the operation.
    /// Return a list of recipients who need to be notified.
    fn apply_operation_as_sender(
        &mut self,
        chain_id: ChainId,
        height: BlockHeight,
        index: usize,
        operation: &Operation,
    ) -> Result<Vec<ChainId>, Error> {
        match &operation {
            Operation::OpenChain {
                id,
                committee,
                admin_id,
                ..
            } => {
                let expected_id = ChainId::child(OperationId {
                    chain_id,
                    height,
                    index,
                });
                ensure!(id == &expected_id, Error::InvalidNewChainId(*id));
                ensure!(
                    Some(admin_id) == self.admin_id(chain_id).as_ref(),
                    Error::InvalidNewChainAdminId(*id)
                );
                ensure!(
                    self.committee.as_ref() == Some(committee),
                    Error::InvalidCommittee
                );
                Ok(vec![*id, *admin_id])
            }
            Operation::ChangeOwner { new_owner } => {
                self.manager = ChainManager::single(*new_owner);
                Ok(Vec::new())
            }
            Operation::ChangeMultipleOwners { new_owners } => {
                self.manager = ChainManager::multiple(new_owners.clone());
                Ok(Vec::new())
            }
            Operation::CloseChain => {
                self.manager = ChainManager::default();
                Ok(Vec::new())
            }
            Operation::Transfer {
                amount, recipient, ..
            } => {
                ensure!(*amount > Amount::zero(), Error::IncorrectTransferAmount);
                ensure!(
                    self.balance >= (*amount).into(),
                    Error::InsufficientFunding {
                        current_balance: self.balance
                    }
                );
                self.balance.try_sub_assign((*amount).into())?;
                match recipient {
                    Address::Burn => Ok(Vec::new()),
                    Address::Account(id) => Ok(vec![*id]),
                }
            }
        }
    }

    /// Execute the recipient's side of an operation.
    /// Returns true if the operation changed the chain state.
    /// Operations must be executed by order of heights in the sender's chain.
    fn apply_operation_as_recipient(
        &mut self,
        chain_id: ChainId,
        operation: &Operation,
    ) -> Result<(), Error> {
        match operation {
            Operation::Transfer { amount, .. } => {
                self.balance = self
                    .balance
                    .try_add((*amount).into())
                    .unwrap_or_else(|_| Balance::max());
                Ok(())
            }
            Operation::OpenChain { id, admin_id, .. } if *admin_id == chain_id => {
                match &mut self.status {
                    Some(ChainStatus::Managing { subscribers }) => {
                        subscribers.push(*id);
                        Ok(())
                    }
                    _ => Err(Error::InvalidCrossChainRequest),
                }
            }
            _ => Ok(()),
        }
    }
}
