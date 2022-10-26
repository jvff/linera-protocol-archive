wit_bindgen_wasmtime::export!("../linera-contracts/api.wit");
wit_bindgen_wasmtime::import!("../linera-contracts/contract.wit");

use self::contract::{
    ApplicationResult, ApplyOperation, Contract, ContractData, PollApplicationResult,
};
use crate::{OperationContext, RawExecutionResult, WritableStorage};
use linera_base::{crypto::IncorrectHashSize, messages::Destination};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use thiserror::Error;
use wasmtime::{Engine, Linker, Module, Store, Trap};

pub struct WasmApplication {}

impl WasmApplication {
    pub fn prepare_runtime(
        &self,
        storage: &dyn WritableStorage,
    ) -> Result<WritableRuntimeContext, PrepareRuntimeError> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);

        // api::add_to_linker(&mut linker, Data::api)?;

        let module = Module::from_file(
            &engine,
            "linera-contracts/example/target/wasm32-unknown-unknown/debug/linera_contract_example.wasm",
        )?;
        let data = Data::default();
        let mut store = Store::new(&engine, data);
        let (contract, _instance) =
            Contract::instantiate(&mut store, &module, &mut linker, Data::contract)?;

        Ok(WritableRuntimeContext { contract, store })
    }
}

pub struct WritableRuntimeContext {
    contract: Contract<Data>,
    store: Store<Data>,
}

impl WritableRuntimeContext {
    pub fn apply_operation(
        mut self,
        context: &OperationContext,
        operation: &[u8],
    ) -> ExternalFuture<ApplyOperation> {
        let future = self
            .contract
            .apply_operation_new(&mut self.store, context.into(), operation);

        ExternalFuture::new(future, self.contract, self.store)
    }
}

#[derive(Default)]
pub struct Data {
    contract: ContractData,
    // api: Api,
    // api_tables: api::ApiTables<Api>,
}

impl Data {
    pub fn contract(&mut self) -> &mut ContractData {
        &mut self.contract
    }
}

#[derive(Debug, Error)]
pub enum PrepareRuntimeError {
    #[error("Failed to instantiate smart contract Wasm module")]
    Instantiate(#[from] wit_bindgen_wasmtime::anyhow::Error),
}

impl From<PrepareRuntimeError> for linera_base::error::Error {
    fn from(error: PrepareRuntimeError) -> Self {
        // TODO
        linera_base::error::Error::UnknownApplication
    }
}

impl<'argument> From<&'argument OperationContext> for contract::OperationContext<'argument> {
    fn from(host: &'argument OperationContext) -> Self {
        contract::OperationContext {
            chain_id: host.chain_id.0.as_bytes().as_slice(),
            height: host.height.0,
            index: host
                .index
                .try_into()
                .expect("Operation index should fit in an `u64`"),
        }
    }
}

pub enum ExternalFuture<Future> {
    FailedToCreate(Trap),
    Active {
        contract: Contract<Data>,
        store: Store<Data>,
        future: Future,
    },
}

impl<Future> ExternalFuture<Future> {
    pub fn new(
        creation_result: Result<Future, Trap>,
        contract: Contract<Data>,
        store: Store<Data>,
    ) -> Self {
        match creation_result {
            Ok(future) => ExternalFuture::Active {
                contract,
                store,
                future,
            },
            Err(trap) => ExternalFuture::FailedToCreate(trap),
        }
    }
}

impl<InnerFuture> Future for ExternalFuture<InnerFuture>
where
    InnerFuture: ExternalFutureInterface + Unpin,
{
    type Output = Result<InnerFuture::Output, linera_base::error::Error>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        match self.get_mut() {
            ExternalFuture::FailedToCreate(_) => {
                Poll::Ready(Err(linera_base::error::Error::UnknownApplication))
            }
            ExternalFuture::Active {
                contract,
                store,
                future,
            } => future.poll(contract, store),
        }
    }
}

pub trait ExternalFutureInterface {
    type Output;

    fn poll(
        &self,
        contract: &Contract<Data>,
        store: &mut Store<Data>,
    ) -> Poll<Result<Self::Output, linera_base::error::Error>>;
}

impl ExternalFutureInterface for ApplyOperation {
    type Output = RawExecutionResult<Vec<u8>>;

    fn poll(
        &self,
        contract: &Contract<Data>,
        store: &mut Store<Data>,
    ) -> Poll<Result<Self::Output, linera_base::error::Error>> {
        match contract.apply_operation_poll(store, self) {
            Ok(PollApplicationResult::Ready(Ok(result))) => Poll::Ready(result.try_into()),
            Ok(PollApplicationResult::Ready(Err(_message))) => {
                Poll::Ready(Err(linera_base::error::Error::UnknownApplication))
            }
            Ok(PollApplicationResult::Pending) => Poll::Pending,
            Err(_) => Poll::Ready(Err(linera_base::error::Error::UnknownApplication)),
        }
    }
}

impl TryFrom<ApplicationResult> for RawExecutionResult<Vec<u8>> {
    type Error = linera_base::error::Error;

    fn try_from(result: ApplicationResult) -> Result<Self, Self::Error> {
        let effects = result
            .effects
            .into_iter()
            .map(|(destination, effect)| Ok((destination.try_into()?, effect)))
            .collect::<Result<_, IncorrectHashSize>>()
            .map_err(|_| linera_base::error::Error::UnknownApplication)?;

        let subscribe = result
            .subscribe
            .into_iter()
            .map(|(channel_id, chain_id)| Ok((channel_id, chain_id.as_slice().try_into()?)))
            .collect::<Result<_, IncorrectHashSize>>()
            .map_err(|_| linera_base::error::Error::UnknownApplication)?;

        let unsubscribe = result
            .unsubscribe
            .into_iter()
            .map(|(channel_id, chain_id)| Ok((channel_id, chain_id.as_slice().try_into()?)))
            .collect::<Result<_, IncorrectHashSize>>()
            .map_err(|_| linera_base::error::Error::UnknownApplication)?;

        Ok(RawExecutionResult {
            effects,
            subscribe,
            unsubscribe,
        })
    }
}

impl TryFrom<contract::Destination> for Destination {
    type Error = IncorrectHashSize;

    fn try_from(guest: contract::Destination) -> Result<Self, Self::Error> {
        Ok(match guest {
            contract::Destination::Recipient(chain_id) => {
                Destination::Recipient(chain_id.as_slice().try_into()?)
            }
            contract::Destination::Subscribers(channel_id) => Destination::Subscribers(channel_id),
        })
    }
}
