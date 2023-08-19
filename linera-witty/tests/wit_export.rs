#[path = "common/test_instance.rs"]
mod test_instance;

#[cfg(feature = "wasmer")]
use self::test_instance::WasmerInstanceFactory;
#[cfg(feature = "wasmtime")]
use self::test_instance::WasmtimeInstanceFactory;
use self::test_instance::{MockInstanceFactory, TestInstanceFactory};
use linera_witty::{wit_export, wit_import, ExportTo, Instance, Runtime, RuntimeMemory};
use test_case::test_case;

#[wit_import(package = "witty-macros:test-modules")]
pub trait Entrypoint {
    fn entrypoint();
}

pub struct SimpleFunction;

#[wit_export(package = "witty-macros:test-modules")]
impl SimpleFunction {
    fn simple() {
        println!("In simple");
    }
}

#[test_case(MockInstanceFactory::default(); "with a mock instance")]
#[cfg_attr(feature = "wasmer", test_case(WasmerInstanceFactory; "with Wasmer"))]
#[cfg_attr(feature = "wasmtime", test_case(WasmtimeInstanceFactory; "with Wasmtime"))]
fn simple_function<InstanceFactory>(mut factory: InstanceFactory)
where
    InstanceFactory: TestInstanceFactory,
    InstanceFactory::Instance: InstanceForEntrypoint,
    <<InstanceFactory::Instance as Instance>::Runtime as Runtime>::Memory:
        RuntimeMemory<InstanceFactory::Instance>,
    SimpleFunction: ExportTo<InstanceFactory::Builder>,
{
    let instance = factory.load_test_module("import", "simple-function", |linker| {
        SimpleFunction::export_to(linker).expect("Failed to export simple function WIT interface")
    });

    Entrypoint::new(instance)
        .entrypoint()
        .expect("Failed to call guest's `simple` function");
}

pub struct Getters;

#[wit_export(package = "witty-macros:test-modules")]
impl Getters {
    fn get_true() -> bool {
        true
    }

    fn get_false() -> bool {
        false
    }

    fn get_s8() -> i8 {
        -125
    }

    fn get_u8() -> u8 {
        200
    }

    fn get_s16() -> i16 {
        -410
    }

    fn get_u16() -> u16 {
        60_000
    }

    fn get_s32() -> i32 {
        -100_000
    }

    fn get_u32() -> u32 {
        3_000_111
    }

    fn get_float32() -> f32 {
        -0.125
    }

    fn get_float64() -> f64 {
        128.25
    }
}

#[test_case(MockInstanceFactory::default(); "with a mock instance")]
#[cfg_attr(feature = "wasmer", test_case(WasmerInstanceFactory; "with Wasmer"))]
#[cfg_attr(feature = "wasmtime", test_case(WasmtimeInstanceFactory; "with Wasmtime"))]
fn getters<InstanceFactory>(mut factory: InstanceFactory)
where
    InstanceFactory: TestInstanceFactory,
    InstanceFactory::Instance: InstanceForEntrypoint,
    <<InstanceFactory::Instance as Instance>::Runtime as Runtime>::Memory:
        RuntimeMemory<InstanceFactory::Instance>,
    Getters: ExportTo<InstanceFactory::Builder>,
{
    let instance = factory.load_test_module("import", "getters", |instance| {
        Getters::export_to(instance).expect("Failed to export getters WIT interface")
    });

    Entrypoint::new(instance)
        .entrypoint()
        .expect("Failed to execute test of imported getters");
}

pub struct Setters;

#[wit_export(package = "witty-macros:test-modules")]
impl Setters {
    #[allow(clippy::bool_assert_comparison)]
    fn set_bool(value: bool) {
        assert_eq!(value, false);
    }

    fn set_s8(value: i8) {
        assert_eq!(value, -100);
    }

    fn set_u8(value: u8) {
        assert_eq!(value, 201);
    }

    fn set_s16(value: i16) {
        assert_eq!(value, -20_000);
    }

    fn set_u16(value: u16) {
        assert_eq!(value, 50_000);
    }

    fn set_s32(value: i32) {
        assert_eq!(value, -2_000_000);
    }

    fn set_u32(value: u32) {
        assert_eq!(value, 4_000_000);
    }

    fn set_float32(value: f32) {
        assert_eq!(value, 10.4);
    }

    fn set_float64(value: f64) {
        assert_eq!(value, -0.000_08);
    }
}

#[test_case(MockInstanceFactory::default(); "with a mock instance")]
#[cfg_attr(feature = "wasmer", test_case(WasmerInstanceFactory; "with Wasmer"))]
#[cfg_attr(feature = "wasmtime", test_case(WasmtimeInstanceFactory; "with Wasmtime"))]
fn setters<InstanceFactory>(mut factory: InstanceFactory)
where
    InstanceFactory: TestInstanceFactory,
    InstanceFactory::Instance: InstanceForEntrypoint,
    <<InstanceFactory::Instance as Instance>::Runtime as Runtime>::Memory:
        RuntimeMemory<InstanceFactory::Instance>,
    Setters: ExportTo<InstanceFactory::Builder>,
{
    let instance = factory.load_test_module("import", "setters", |instance| {
        Setters::export_to(instance).expect("Failed to export setters WIT interface")
    });

    Entrypoint::new(instance)
        .entrypoint()
        .expect("Failed to execute test of imported setters");
}

pub struct Operations;

#[wit_export(package = "witty-macros:test-modules")]
impl Operations {
    fn and_bool(first: bool, second: bool) -> bool {
        first && second
    }

    fn add_s8(first: i8, second: i8) -> i8 {
        first + second
    }

    fn add_u8(first: u8, second: u8) -> u8 {
        first + second
    }

    fn add_s16(first: i16, second: i16) -> i16 {
        first + second
    }

    fn add_u16(first: u16, second: u16) -> u16 {
        first + second
    }

    fn add_s32(first: i32, second: i32) -> i32 {
        first + second
    }

    fn add_u32(first: u32, second: u32) -> u32 {
        first + second
    }

    fn add_s64(first: i64, second: i64) -> i64 {
        first + second
    }

    fn add_u64(first: u64, second: u64) -> u64 {
        first + second
    }

    fn add_float32(first: f32, second: f32) -> f32 {
        first + second
    }

    fn add_float64(first: f64, second: f64) -> f64 {
        first + second
    }
}

#[test_case(MockInstanceFactory::default(); "with a mock instance")]
#[cfg_attr(feature = "wasmer", test_case(WasmerInstanceFactory; "with Wasmer"))]
#[cfg_attr(feature = "wasmtime", test_case(WasmtimeInstanceFactory; "with Wasmtime"))]
fn operations<InstanceFactory>(mut factory: InstanceFactory)
where
    InstanceFactory: TestInstanceFactory,
    InstanceFactory::Instance: InstanceForEntrypoint,
    <<InstanceFactory::Instance as Instance>::Runtime as Runtime>::Memory:
        RuntimeMemory<InstanceFactory::Instance>,
    Operations: ExportTo<InstanceFactory::Builder>,
{
    let instance = factory.load_test_module("import", "operations", |instance| {
        Operations::export_to(instance).expect("Failed to export operations WIT interface")
    });

    Entrypoint::new(instance)
        .entrypoint()
        .expect("Failed to execute test of imported operations");
}
