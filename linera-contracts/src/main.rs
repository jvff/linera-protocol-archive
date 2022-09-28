#[cfg(feature = "wasmer")]
use wasmer::{imports, Instance, Module, Store, Value};
#[cfg(feature = "wasmtime")]
use wasmtime::{Engine, Linker, Module, Store};

#[cfg(feature = "wasmtime")]
wit_bindgen_host_wasmtime_rust::import!("contract.wit");

#[cfg(feature = "wasmer")]
fn main() -> Result<(), anyhow::Error> {
    todo!();
    let store = Store::default();
    let module = Module::from_file(
        &store,
        "example/target/wasm32-unknown-unknown/debug/linera_contract_example.wasm",
    )?;
    let import_object = imports! {};
    let instance = Instance::new(&module, &import_object)?;
    let entry_point = instance.exports.get_function("example")?;

    for _ in 0..3 {
        let result = entry_point.call(&[])?;

        println!("{result:?}");
    }

    Ok(())
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
    let (contract, instance) =
        contract::Contract::instantiate(&mut store, &module, &mut linker, |data| data)?;

    loop {
        match contract.example(&mut store)? {
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
