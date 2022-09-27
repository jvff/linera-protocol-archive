#[cfg(feature = "wasmer")]
use wasmer::{imports, Instance, Module, Store, Value};
#[cfg(feature = "wasmtime")]
use wasmtime::{Engine, Instance, Module, Store};

#[cfg(feature = "wasmer")]
fn main() -> Result<(), anyhow::Error> {
    let store = Store::default();
    let module = Module::from_file(
        &store,
        "example/target/wasm32-unknown-unknown/debug/linera_contract_example.wasm",
    )?;
    let import_object = imports! {};
    let instance = Instance::new(&module, &import_object)?;
    let entry_point = instance.exports.get_function("example")?;

    for _ in 0..3 {
        let result = entry_point.call(&[Value::I32(100)])?;

        println!("{result:?}");
    }

    Ok(())
}

#[cfg(feature = "wasmtime")]
fn main() -> Result<(), anyhow::Error> {
    let engine = Engine::default();
    let module = Module::from_file(
        &engine,
        "example/target/wasm32-unknown-unknown/debug/linera_contract_example.wasm",
    )?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let entry_point = instance.get_typed_func::<(u32,), (u32,), _>(&mut store, "example")?;

    for _ in 0..3 {
        let result = entry_point.call(&mut store, (50,))?.0;

        println!("{result}");
    }

    Ok(())
}
