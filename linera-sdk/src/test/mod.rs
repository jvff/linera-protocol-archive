mod conversions;

use crate::{
    base::{BytecodeId, EffectId},
    crypto::PublicKey,
    ApplicationId, BlockHeight, ChainId,
};
use cargo_toml::Manifest;
use dashmap::DashMap;
use linera_base::{
    committee::Committee,
    crypto as base_crypto,
    data_types::{self as base},
};
use linera_chain::data_types as chain;
use linera_core::{
    data_types as core,
    worker::{ValidatorWorker, WorkerState},
};
use linera_execution::{
    self as execution,
    system::{SystemChannel, SystemEffect, SystemOperation},
    Bytecode, Destination, WasmRuntime,
};
use linera_storage::{MemoryStoreClient, Store};
use serde::Serialize;
use std::{
    mem,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::sync::Mutex;

pub struct TestValidator {
    key_pair: base_crypto::KeyPair,
    committee: Committee,
    worker: Arc<Mutex<WorkerState<MemoryStoreClient>>>,
    root_chain_counter: Arc<AtomicUsize>,
    chains: Arc<DashMap<ChainId, ActiveChain>>,
}

impl Default for TestValidator {
    fn default() -> Self {
        let key_pair = base_crypto::KeyPair::generate();
        let committee = Committee::make_simple(vec![base::ValidatorName(key_pair.public())]);
        let client = MemoryStoreClient::new(Some(WasmRuntime::default()));

        let worker = WorkerState::new(
            "Single validator node".to_string(),
            Some(key_pair.copy()),
            client,
        );

        TestValidator {
            key_pair,
            committee,
            worker: Arc::new(Mutex::new(worker)),
            root_chain_counter: Arc::default(),
            chains: Arc::default(),
        }
    }
}

impl Clone for TestValidator {
    fn clone(&self) -> Self {
        TestValidator {
            key_pair: self.key_pair.copy(),
            committee: self.committee.clone(),
            worker: self.worker.clone(),
            root_chain_counter: self.root_chain_counter.clone(),
            chains: self.chains.clone(),
        }
    }
}

impl TestValidator {
    pub async fn with_current_bytecode() -> (TestValidator, BytecodeId) {
        let validator = TestValidator::default();
        let publisher = validator.new_chain().await;

        let bytecode_id = publisher.publish_current_bytecode().await;

        (validator, bytecode_id)
    }

    pub async fn with_current_application(
        parameters: Vec<u8>,
        initialization_argument: Vec<u8>,
    ) -> (TestValidator, ApplicationId) {
        let (validator, bytecode_id) = TestValidator::with_current_bytecode().await;

        let mut creator = validator.new_chain().await;

        let application_id = creator
            .create_application(bytecode_id, parameters, initialization_argument, vec![])
            .await;

        (validator, application_id)
    }

    pub async fn new_chain(&self) -> ActiveChain {
        let key_pair = base_crypto::KeyPair::generate();
        let description =
            base::ChainDescription::Root(self.root_chain_counter.fetch_add(1, Ordering::AcqRel));

        self.worker
            .lock()
            .await
            .storage_client()
            .create_chain(
                self.committee.clone(),
                base::ChainId::root(0),
                description,
                key_pair.public(),
                0.into(),
                base::Timestamp::from(0),
            )
            .await
            .expect("Failed to create chain");

        let chain = ActiveChain {
            key_pair,
            description,
            tip: Arc::default(),
            validator: self.clone(),
        };

        self.chains.insert(description.into(), chain.clone());

        chain
    }

    pub fn get_chain(&self, chain_id: &ChainId) -> ActiveChain {
        self.chains.get(chain_id).expect("Chain not found").clone()
    }
}

pub struct ActiveChain {
    key_pair: base_crypto::KeyPair,
    description: base::ChainDescription,
    tip: Arc<Mutex<Option<chain::Certificate>>>,
    validator: TestValidator,
}

impl Clone for ActiveChain {
    fn clone(&self) -> Self {
        ActiveChain {
            key_pair: self.key_pair.copy(),
            description: self.description,
            tip: self.tip.clone(),
            validator: self.validator.clone(),
        }
    }
}

impl ActiveChain {
    pub fn id(&self) -> ChainId {
        self.description.into()
    }

    pub fn public_key(&self) -> PublicKey {
        self.key_pair.public().into()
    }

    pub async fn add_block(&self, block_builder: impl FnOnce(&mut Block)) {
        let mut tip = self.tip.lock().await;
        let mut block = Block::new(
            self.description.into(),
            self.key_pair.public().into(),
            tip.as_ref(),
            self.validator.clone(),
        );

        block_builder(&mut block);

        let certificate = block.sign(&self.validator).await;

        self.validator
            .worker
            .lock()
            .await
            .fully_handle_certificate(certificate.clone(), vec![])
            .await
            .expect("Rejected certificate");

        *tip = Some(certificate);
    }

    pub async fn handle_received_effects(&self) {
        let chain_id = self.id().into();
        let (information, _) = self
            .validator
            .worker
            .lock()
            .await
            .handle_chain_info_query(core::ChainInfoQuery::new(chain_id).with_pending_messages())
            .await
            .expect("Failed to query chain's pending messages");
        let messages = information.info.requested_pending_messages;

        self.add_block(|block| {
            block.with_raw_messages(messages);
        })
        .await;
    }

    pub async fn publish_current_bytecode(&self) -> BytecodeId {
        Self::build_bytecodes();
        let (contract, service) = self.find_current_bytecodes().await;

        self.add_block(|block| {
            block.with_system_operation(SystemOperation::PublishBytecode { contract, service });
        })
        .await;

        let publish_effect_id = EffectId {
            chain_id: self.description.into(),
            height: self.tip_height().await.into(),
            index: 0,
        };

        self.add_block(|block| {
            block.with_message(publish_effect_id);
        })
        .await;

        BytecodeId(publish_effect_id)
    }

    fn build_bytecodes() {
        let output = std::process::Command::new("cargo")
            .args(["build", "--release", "--target", "wasm32-unknown-unknown"])
            .output()
            .expect("Failed to build WASM binaries");

        if !output.status.success() {
            panic!(
                "Failed to build bytecode binaries.\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    async fn find_current_bytecodes(&self) -> (Bytecode, Bytecode) {
        let mut cargo_manifest =
            Manifest::from_path("Cargo.toml").expect("Failed to load Cargo.toml manifest");

        cargo_manifest
            .complete_from_path(Path::new("."))
            .expect("Failed to populate manifest with information inferred from the repository");

        let binaries: Vec<_> = cargo_manifest
            .bin
            .into_iter()
            .filter_map(|binary| binary.name)
            .filter(|name| name.contains("service") || name.contains("contract"))
            .collect();

        assert_eq!(
            binaries.len(),
            2,
            "Could not figure out contract and service bytecode binaries.\
            Please specify them manually using `publish_bytecode`."
        );

        let (contract_binary, service_binary) =
            if binaries[0].contains("service") && !binaries[0].contains("contract") {
                (&binaries[1], &binaries[0])
            } else {
                (&binaries[0], &binaries[1])
            };

        let base_path = PathBuf::from("../target/wasm32-unknown-unknown/release");
        let contract_path = base_path.join(format!("{}.wasm", contract_binary));
        let service_path = base_path.join(format!("{}.wasm", service_binary));

        (
            Bytecode::load_from_file(contract_path)
                .await
                .expect("Failed to load contract bytecode from file"),
            Bytecode::load_from_file(service_path)
                .await
                .expect("Failed to load service bytecode from file"),
        )
    }

    async fn tip_height(&self) -> base::BlockHeight {
        self.tip
            .lock()
            .await
            .as_ref()
            .expect("Block was not successfully added")
            .value
            .block()
            .height
    }

    pub async fn subscribe_to_published_bytecodes_from(&mut self, publisher_id: ChainId) {
        let publisher = self.validator.get_chain(&publisher_id);

        self.add_block(|block| {
            block.with_system_operation(SystemOperation::Subscribe {
                chain_id: publisher.id().into(),
                channel: SystemChannel::PublishedBytecodes,
            });
        })
        .await;

        let effect_id = EffectId {
            chain_id: self.description.into(),
            height: self.tip_height().await.into(),
            index: 0,
        };

        publisher
            .add_block(|block| {
                block.with_message(effect_id);
            })
            .await;

        let effect_id = EffectId {
            chain_id: publisher.description.into(),
            height: publisher.tip_height().await.into(),
            index: 0,
        };

        self.add_block(|block| {
            block.with_message(effect_id);
        })
        .await;
    }

    pub async fn create_application(
        &mut self,
        bytecode_id: BytecodeId,
        parameters: Vec<u8>,
        initialization_argument: Vec<u8>,
        required_application_ids: Vec<ApplicationId>,
    ) -> ApplicationId {
        let bytecode_location_effect = if self.needs_bytecode_location(bytecode_id).await {
            self.subscribe_to_published_bytecodes_from(bytecode_id.0.chain_id)
                .await;
            Some(self.find_bytecode_location(bytecode_id).await)
        } else {
            None
        };

        let required_application_ids = required_application_ids
            .into_iter()
            .map(|id| id.into())
            .collect();

        self.add_block(|block| {
            if let Some(effect_id) = bytecode_location_effect {
                block.with_message(effect_id);
            }

            block.with_system_operation(SystemOperation::CreateApplication {
                bytecode_id: bytecode_id.into(),
                parameters,
                initialization_argument,
                required_application_ids,
            });
        })
        .await;

        let creation_effect_id = EffectId {
            chain_id: self.description.into(),
            height: self.tip_height().await.into(),
            index: 0,
        };

        ApplicationId {
            bytecode: bytecode_id,
            creation: creation_effect_id,
        }
    }

    async fn needs_bytecode_location(&self, bytecode_id: BytecodeId) -> bool {
        let applications = self
            .validator
            .worker
            .lock()
            .await
            .get_application_registry(self.id().into())
            .await
            .expect("Failed to load application registry");

        applications
            .bytecode_locations_for([bytecode_id.into()])
            .await
            .expect("Failed to check known bytecode locations")
            .is_empty()
    }

    async fn find_bytecode_location(&self, bytecode_id: BytecodeId) -> EffectId {
        let bytecode_id = execution::BytecodeId::from(bytecode_id);
        let worker = self.validator.worker.lock().await;

        for height in bytecode_id.0.height.0.. {
            let certificate = worker
                .get_certificate(bytecode_id.0.chain_id, height.into())
                .await
                .expect("Failed to load certificate to search for bytecode location")
                .expect("Bytecode location not found");

            let effect_index = certificate.value.effects().iter().position(|effect| {
                matches!(
                    &effect.effect,
                    execution::Effect::System(SystemEffect::BytecodeLocations { locations })
                        if locations.iter().any(|(id, _)| id == &bytecode_id)
                )
            });

            if let Some(index) = effect_index {
                return EffectId {
                    chain_id: bytecode_id.0.chain_id.into(),
                    height: BlockHeight(height),
                    index: index.try_into().expect(
                        "Incompatible `EffectId` index types in \
                        `linera-sdk` and `linera-execution`",
                    ),
                };
            }
        }

        panic!("Bytecode not found in the chain it was supposed to be published on");
    }

    pub async fn query(&self, application: ApplicationId, query: Vec<u8>) -> Vec<u8> {
        let response = self
            .validator
            .worker
            .lock()
            .await
            .query_application(
                self.id().into(),
                application.into(),
                &execution::Query::User(query),
            )
            .await
            .expect("Failed to query application");

        match response {
            execution::Response::User(bytes) => bytes,
            execution::Response::System(_) => unreachable!("User query returned a system response"),
        }
    }
}

pub struct Block {
    block: chain::Block,
    incoming_effects: Vec<EffectId>,
    validator: TestValidator,
}

impl Block {
    fn new(
        chain_id: ChainId,
        owner: base::Owner,
        previous_block: Option<&chain::Certificate>,
        validator: TestValidator,
    ) -> Self {
        let previous_block_hash = previous_block.map(|certificate| certificate.value.hash());
        let height = previous_block
            .and_then(|certificate| certificate.value.block().height.try_add_one().ok())
            .unwrap_or_default();

        Block {
            block: chain::Block {
                epoch: 0.into(),
                chain_id: chain_id.into(),
                incoming_messages: vec![],
                operations: vec![],
                previous_block_hash,
                height,
                authenticated_signer: Some(owner),
                timestamp: base::Timestamp::from(0),
            },
            incoming_effects: Vec::new(),
            validator,
        }
    }

    fn with_system_operation(&mut self, operation: SystemOperation) -> &mut Self {
        self.block
            .operations
            .push((linera_execution::ApplicationId::System, operation.into()));
        self
    }

    pub fn with_operation(&mut self, application: ApplicationId, operation: Vec<u8>) -> &mut Self {
        self.block
            .operations
            .push((application.into(), operation.into()));
        self
    }

    pub fn with_message(&mut self, effect_id: EffectId) -> &mut Self {
        self.incoming_effects.push(effect_id);
        self
    }

    fn with_raw_messages(
        &mut self,
        messages: impl IntoIterator<Item = chain::Message>,
    ) -> &mut Self {
        self.block.incoming_messages.extend(messages);
        self
    }

    async fn sign(mut self, validator: &TestValidator) -> chain::Certificate {
        self.collect_incoming_effects().await;

        let (effects, info) = validator
            .worker
            .lock()
            .await
            .stage_block_execution(&self.block)
            .await
            .expect("Failed to execute block");
        let state_hash = info.info.state_hash.expect("Missing execution state hash");

        let value = chain::HashedValue::new_confirmed(self.block, effects, state_hash);
        let vote = chain::LiteVote::new(value.lite(), &validator.key_pair);
        let mut builder = chain::SignatureAggregator::new(value, &validator.committee);
        builder
            .append(vote.validator, vote.signature)
            .unwrap()
            .unwrap()
    }

    async fn collect_incoming_effects(&mut self) {
        for effect_id in mem::take(&mut self.incoming_effects) {
            let message = self.build_message(effect_id).await;
            self.block.incoming_messages.push(message);
        }
    }

    async fn build_message(&mut self, effect_id: EffectId) -> chain::Message {
        let certificate = self
            .validator
            .worker
            .lock()
            .await
            .get_certificate(effect_id.chain_id.into(), effect_id.height.into())
            .await
            .expect("Failed to load certificate with effect to be received")
            .expect("Certificate with effect to be received was not (yet?) created");

        let outgoing_effect = certificate
            .value
            .effects()
            .get(effect_id.index as usize)
            .expect("Missing effect")
            .clone();

        let origin = chain::Origin {
            sender: effect_id.chain_id.into(),
            medium: match outgoing_effect.destination {
                Destination::Recipient(_) => chain::Medium::Direct,
                Destination::Subscribers(channel) => chain::Medium::Channel(channel),
            },
        };

        let event = chain::Event {
            certificate_hash: certificate.value.hash(),
            height: effect_id.height.into(),
            index: effect_id.index as usize,
            authenticated_signer: None,
            timestamp: base::Timestamp::from(0),
            effect: outgoing_effect.effect,
        };

        chain::Message {
            application_id: outgoing_effect.application_id,
            origin,
            event,
        }
    }
}

pub trait ToBcsBytes {
    fn to_bcs_bytes(&self) -> Vec<u8>;
}

impl<T> ToBcsBytes for T
where
    T: Serialize,
{
    fn to_bcs_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(self).expect("Failed to serialize")
    }
}
