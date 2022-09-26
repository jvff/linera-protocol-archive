use wasmtime::{Engine, Instance, Module, Store};

fn main() -> Result<(), anyhow::Error> {
    let engine = Engine::default();
    let module = Module::from_file(
        &engine,
        "example/target/wasm32-unknown-unknown/debug/linera_contract_example.wasm",
    )?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let entry_point = instance.get_typed_func::<(u32,), (u32,), _>(&mut store, "example")?;

    let result = entry_point.call(&mut store, (50,))?.0;

    println!("{result}");

    Ok(())
}
