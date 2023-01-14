use anyhow::{bail, Result};
use wasmtime::*;

fn main() -> Result<()> {
    let mut report = TestReport::default();
    let engine = Engine::default();
    let test_module = load_test_module(&engine)?;
    let tests: Vec<_> = test_module.exports().filter_map(Test::new).collect();

    eprintln!("\nrunning {} tests", tests.len());

    for test in tests {
        test.run(&mut report, &engine, &test_module)?;
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

    pub fn run(self, report: &mut TestReport, engine: &Engine, test_module: &Module) -> Result<()> {
        eprint!("test {} ...", self.name);

        if self.ignore {
            report.ignore();
        } else {
            let mut store = Store::new(&engine, ());
            let instance = Instance::new(&mut store, &test_module, &[])?;

            let function = instance.get_typed_func::<(), (), _>(&mut store, self.function)?;

            report.result(function.call(&mut store, ()));
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct TestReport {
    passed: usize,
    failed: usize,
    ignored: usize,
}

impl TestReport {
    pub fn result<T, E>(&mut self, result: Result<T, E>) {
        if result.is_ok() {
            self.pass();
        } else {
            self.fail();
        }
    }

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
