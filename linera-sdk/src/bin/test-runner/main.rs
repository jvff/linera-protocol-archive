use anyhow::{bail, Result};
use wasmtime::*;

fn main() -> Result<()> {
    let engine = Engine::default();
    let test_module = load_test_module(&engine)?;
    let mut tests = Vec::new();
    for export in test_module.exports() {
        if let Some(test) = Test::new(export) {
            tests.push(test);
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

impl<'a> Test<'a> {
    pub fn new(export: ExportType<'a>) -> Option<Self> {
        let function = export.name();
        let test_name = function.strip_prefix("$webassembly-test$")?;
        let ignored_test_name = test_name.strip_prefix("ignore$");

        Some(Test {
            function,
            name: ignored_test_name.unwrap_or(test_name),
            ignore: ignored_test_name.is_some(),
        })
    }
}
