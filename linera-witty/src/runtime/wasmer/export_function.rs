#![allow(clippy::let_unit_value)]

use {
    super::{InstanceBuilder, InstanceSlot},
    crate::{primitive_types::MaybeFlatType, ExportFunction, RuntimeError},
    std::error::Error,
    wasmer::{FromToNativeWasmType, Function, FunctionEnvMut, WasmTypeList},
};

macro_rules! export_function {
    ($( $names:ident: $types:ident ),* $(,)*) => {
        export_function!(| $( $names: $types ),*);
    };

    ($( $names:ident: $types:ident ),* |) => {
        export_function!(@generate $( $names: $types ),*);
    };

    (
        $( $names:ident: $types:ident ),*
        | $next_name:ident: $next_type:ident
        $(, $queued_names:ident: $queued_types:ident )*
    ) => {
        export_function!(@generate $( $names: $types ),*);
        export_function!(
            $( $names: $types, )* $next_name: $next_type
            | $( $queued_names: $queued_types ),*
        );
    };

    (@generate $( $names:ident: $types:ident ),*) => {
        impl<Handler, HandlerError, $( $types, )* FlatResult>
            ExportFunction<Handler, ($( $types, )*), FlatResult> for InstanceBuilder
        where
            $( $types: FromToNativeWasmType, )*
            FlatResult: MaybeFlatType + WasmTypeList,
            HandlerError: Error + Send + Sync + 'static,
            Handler:
                Fn(
                    FunctionEnvMut<'_, InstanceSlot>,
                    ($( $types, )*),
                ) -> Result<FlatResult, HandlerError>
                + Send
                + Sync
                + 'static,
        {
            fn export(
                &mut self,
                module_name: &str,
                function_name: &str,
                handler: Handler,
            ) -> Result<(), RuntimeError> {
                let environment = self.environment();

                let function = Function::new_typed_with_env(
                    self,
                    &environment,
                    move |
                        environment: FunctionEnvMut<'_, InstanceSlot>,
                        $( $names: $types ),*
                    | -> Result<FlatResult, wasmer::RuntimeError> {
                        handler(environment, ($( $names, )*))
                            .map_err(|error| -> Box<dyn std::error::Error + Send + Sync> {
                                Box::new(error)
                            })
                            .map_err(wasmer::RuntimeError::user)
                    },
                );

                self.define(
                    module_name,
                    function_name,
                    function,
                );

                Ok(())
            }
        }
    };
}

export_function!(
    a: A,
    b: B,
    c: C,
    d: D,
    e: E,
    f: F,
    g: G,
    h: H,
    i: I,
    j: J,
    k: K,
    l: L,
    m: M,
    n: N,
    o: O,
    p: P,
);
