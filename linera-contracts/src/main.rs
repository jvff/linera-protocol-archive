#[cfg(feature = "wasmer")]
use wasmer::{imports, Module};
#[cfg(feature = "wasmtime")]
use wasmtime::{Engine, Linker, Module};

#[cfg(feature = "wasmer")]
wit_bindgen_wasmer::import!("contract.wit");
#[cfg(feature = "wasmtime")]
wit_bindgen_wasmtime::import!("contract.wit");

#[cfg(feature = "wasmer")]
type Contract = contract::Contract;
#[cfg(feature = "wasmer")]
type Store = wasmer::Store;

#[cfg(feature = "wasmtime")]
type Contract = contract::Contract<contract::ContractData>;
#[cfg(feature = "wasmtime")]
type Store = wasmtime::Store<contract::ContractData>;

#[cfg(feature = "wasmer")]
fn main() -> Result<(), anyhow::Error> {
    let mut store = Store::default();
    let module = Module::from_file(
        &store,
        "example/target/wasm32-unknown-unknown/debug/linera_contract_example.wasm",
    )?;
    let mut imports = imports! {};
    let (contract, _instance) = contract::Contract::instantiate(&mut store, &module, &mut imports)?;

    run_contract(contract, &mut store)
}

#[cfg(feature = "wasmtime")]
fn main() -> Result<(), anyhow::Error> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    let module = Module::from_file(
        &engine,
        "example/target/wasm32-unknown-unknown/debug/linera_contract_example.wasm",
    )?;
    let data = contract::ContractData {};
    let mut store = Store::new(&engine, data);
    let (contract, _instance) =
        contract::Contract::instantiate(&mut store, &module, &mut linker, |data| data)?;

    run_contract(contract, &mut store)
}

fn run_contract(contract: Contract, store: &mut Store) -> Result<(), anyhow::Error> {
    loop {
        match contract.example(&mut *store)? {
            contract::Poll::Pending => {
                println!("pending");
                continue;
            }
            contract::Poll::Ready(result) => {
                println!("{result}");
                break;
            }
        }
    }

    Ok(())
}
