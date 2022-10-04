use {
    pin_project::pin_project,
    std::{
        fmt::{self, Debug, Formatter},
        future::Future,
        marker::PhantomData,
        mem,
        pin::Pin,
        sync::Arc,
        task::{Context, Poll},
        time::Duration,
    },
    tokio::{sync::Mutex, time::sleep},
};

#[cfg(feature = "wasmer")]
use wasmer::{imports, Module};
#[cfg(feature = "wasmtime")]
use wasmtime::{Engine, Linker, Module};

#[cfg(feature = "wasmer")]
wit_bindgen_wasmer::import!("contract.wit");
#[cfg(feature = "wasmtime")]
wit_bindgen_wasmtime::import!("contract.wit");

#[cfg(feature = "wasmer")]
wit_bindgen_wasmer::export!("api.wit");
#[cfg(feature = "wasmtime")]
wit_bindgen_wasmtime::export!("api.wit");

#[cfg(feature = "wasmer")]
type Contract = contract::Contract;
#[cfg(feature = "wasmer")]
type Store = wasmer::Store;

#[cfg(feature = "wasmtime")]
type Contract = contract::Contract<Data>;
#[cfg(feature = "wasmtime")]
type Store = wasmtime::Store<Data>;

#[cfg(feature = "wasmer")]
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut store = Store::default();
    let module = Module::from_file(
        &store,
        "example/target/wasm32-unknown-unknown/debug/linera_contract_example.wasm",
    )?;
    let mut imports = imports! {};
    let context = Arc::new(Mutex::new(None));
    let api = Api {
        context: context.clone(),
    };
    let api_setup = api::add_to_imports(&mut store, &mut imports, api);
    let (contract, instance) = contract::Contract::instantiate(&mut store, &module, &mut imports)?;

    api_setup(&instance, &store)?;

    run_contract(contract, &mut store, context).await
}

#[cfg(feature = "wasmtime")]
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);

    api::add_to_linker(&mut linker, Data::api)?;

    let module = Module::from_file(
        &engine,
        "example/target/wasm32-unknown-unknown/debug/linera_contract_example.wasm",
    )?;
    let data = Data::default();
    let context = data.context();
    let mut store = Store::new(&engine, data);
    let (contract, _instance) =
        contract::Contract::instantiate(&mut store, &module, &mut linker, Data::contract)?;

    run_contract(contract, &mut store, context).await
}

async fn run_contract(
    contract: Contract,
    store: &mut Store,
    context: Arc<Mutex<Option<&'static mut Context<'static>>>>,
) -> Result<(), anyhow::Error> {
    ContractFuture {
        contract,
        store,
        context,
    }
    .await
}

#[pin_project(project = ContractFutureProjection)]
pub struct ContractFuture<'store> {
    contract: Contract,
    store: &'store mut Store,
    context: Arc<Mutex<Option<&'static mut Context<'static>>>>,
}

impl ContractFutureProjection<'_, '_> {
    fn set_context<'context>(&self, context: &'context mut Context) -> ContextGuard<'context> {
        let mut context_reference = self
            .context
            .try_lock()
            .expect("Unexpected concurrent contract execution");

        *context_reference = Some(unsafe { mem::transmute(context) });

        ContextGuard {
            context: self.context.clone(),
            lifetime: PhantomData,
        }
    }
}

struct ContextGuard<'context> {
    context: Arc<Mutex<Option<&'static mut Context<'static>>>>,
    lifetime: PhantomData<&'context mut ()>,
}

impl Drop for ContextGuard<'_> {
    fn drop(&mut self) {
        let mut context_reference = self
            .context
            .try_lock()
            .expect("Unexpected concurrent contract execution");

        *context_reference = None;
    }
}

impl Future for ContractFuture<'_> {
    type Output = Result<(), anyhow::Error>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        let _context_guard = this.set_context(context);

        match this.contract.example(&mut *this.store) {
            Ok(contract::Poll::Pending) => Poll::Pending,
            Ok(contract::Poll::Ready(value)) => {
                println!("{value}");
                Poll::Ready(Ok(()))
            }
            Err(error) => Poll::Ready(Err(error.into())),
        }
    }
}

#[cfg(feature = "wasmtime")]
#[derive(Default)]
pub struct Data {
    contract: contract::ContractData,
    api: Api,
    api_tables: api::ApiTables<Api>,
}

#[cfg(feature = "wasmtime")]
impl Data {
    pub fn contract(&mut self) -> &mut contract::ContractData {
        &mut self.contract
    }

    pub fn api(&mut self) -> (&mut Api, &mut api::ApiTables<Api>) {
        (&mut self.api, &mut self.api_tables)
    }

    pub fn context(&self) -> Arc<Mutex<Option<&'static mut Context<'static>>>> {
        self.api.context.clone()
    }
}

#[derive(Default)]
pub struct Api {
    context: Arc<Mutex<Option<&'static mut Context<'static>>>>,
}

impl api::Api for Api {
    type Exported = Exported;

    fn exported_new(&mut self, value: u32) -> Self::Exported {
        Exported(Mutex::new(Box::pin(exported(value))))
    }

    fn exported_poll(&mut self, this: &Self::Exported) -> api::Poll {
        let mut context_reference = self
            .context
            .try_lock()
            .expect("Unexpected concurrent contract execution");
        let context = context_reference
            .as_mut()
            .expect("Contract called without a poll context");

        let mut future = this
            .0
            .try_lock()
            .expect("Contract can't call the future concurrently because it's single threaded");

        match future.as_mut().poll(context) {
            Poll::Ready(result) => api::Poll::Ready(result),
            Poll::Pending => api::Poll::Pending,
        }
    }
}

pub struct Exported(Mutex<Pin<Box<dyn Future<Output = u32>>>>);

impl Debug for Exported {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("Exported").field(&"..").finish()
    }
}

async fn exported(value: u32) -> u32 {
    sleep(Duration::from_secs(5)).await;
    value + 1
}
