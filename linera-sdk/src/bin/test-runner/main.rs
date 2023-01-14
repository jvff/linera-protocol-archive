use anyhow::{bail, Result};
use wasmtime::*;

fn main() -> Result<()> {
    let engine = Engine::default();
    let test_module = load_test_module(&engine)?;
    let tests: Vec<_> = test_module.exports().filter_map(Test::new).collect();

    eprintln!("\nrunning {} tests", tests.len());
    let mut report = TestReport::default();
    for test in tests {
        eprint!("test {} ...", test.name);
        if test.ignore {
            report.ignore();
        } else {
            let mut store = Store::new(&engine, ());
            let instance = Instance::new(&mut store, &test_module, &[])?;
            let f = instance.get_typed_func::<(), (), _>(&mut store, test.function)?;

            let pass = f.call(&mut store, ()).is_ok();
            if pass {
                report.pass();
            } else {
                report.fail();
            }
        }
    }
    report.print();
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

#[derive(Clone, Copy, Debug, Default)]
struct TestReport {
    passed: usize,
    failed: usize,
    ignored: usize,
}

impl TestReport {
    pub fn pass(&mut self) {
        self.passed += 1;
        eprintln!(" ok")
    }

    pub fn fail(&mut self) {
        self.failed += 1;
        eprintln!(" FAILED")
    }

    pub fn ignore(&mut self) {
        self.ignored += 1;
        eprintln!(" ignored")
    }

    pub fn print(self) {
        let TestReport {
            passed,
            failed,
            ignored,
        } = self;

        let status = if failed > 0 { "FAILED" } else { "ok" };

        eprintln!("\ntest result: {status}. {passed} passed; {failed} failed; {ignored} ignored;",);
    }
}
