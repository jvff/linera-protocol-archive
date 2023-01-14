use anyhow::{bail, Result};
use wasmtime::*;

fn main() -> Result<()> {
    let engine = Engine::default();
    let test_module = load_test_module(&engine)?;
    let mut tests = Vec::new();
    for export in test_module.exports() {
        let function = export.name();

        if let Some(name) = function.strip_prefix("$webassembly-test$") {
            let mut ignore = true;
            let name = name.strip_prefix("ignore$").unwrap_or_else(|| {
                ignore = false;
                name
            });
            tests.push(TestMeta {
                name,
                function,
                ignore,
            });
        }
    }
    let total = tests.len();

    eprintln!("\nrunning {} tests", total);
    let mut store = Store::new(&engine, ());
    let mut instance = Instance::new(&mut store, &test_module, &[])?;
    let mut passed = 0;
    let mut failed = 0;
    let mut ignored = 0;
    for test in tests {
        eprint!("test {} ...", test.name);
        if test.ignore {
            ignored += 1;
            eprintln!(" ignored")
        } else {
            let f = instance.get_typed_func::<(), (), _>(&mut store, test.function)?;

            let pass = f.call(&mut store, ()).is_ok();
            if pass {
                passed += 1;
                eprintln!(" ok")
            } else {
                // Reset instance on test failure. WASM uses `panic=abort`, so
                // `Drop`s are not called after test failures, and a failed test
                // might leave an instance in an inconsistent state.
                store = Store::new(&engine, ());
                instance = Instance::new(&mut store, &test_module, &[])?;

                failed += 1;
                eprintln!(" FAILED")
            }
        }
    }
    eprintln!(
        "\ntest result: {}. {} passed; {} failed; {} ignored;",
        if failed > 0 { "FAILED" } else { "ok" },
        passed,
        failed,
        ignored,
    );
    Ok(())
}

fn load_test_module(engine: &Engine) -> Result<Module> {
    let module_path = parse_args()?;
    let module = Module::from_file(engine, &module_path)?;
    Ok(module)
}

fn parse_args() -> Result<String> {
    match std::env::args().nth(1) {
        Some(file_path) => Ok(file_path),
        None => {
            bail!("usage: test-runner tests.wasm");
        }
    }
}

struct Test<'a> {
    name: &'a str,
    function: &'a str,
    ignore: bool,
}
