#![allow(clippy::let_unit_value)]

use {
    crate::{primitive_types::MaybeFlatType, ExportFunction, RuntimeError},
    wasmtime::{Caller, Linker, Trap, WasmRet, WasmTy},
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
        impl<Handler, $( $types, )* FlatResult, Data>
            ExportFunction<Handler, ($( $types, )*), FlatResult> for Linker<Data>
        where
            $( $types: WasmTy, )*
            FlatResult: MaybeFlatType + WasmRet,
            Handler:
                Fn(Caller<'_, Data>, ($( $types, )*)) -> Result<FlatResult, RuntimeError>
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
                self.func_wrap(
                    module_name,
                    function_name,
                    move |
                        caller: Caller<'_, Data>,
                        $( $names: $types ),*
                    | -> Result<FlatResult, Trap> {
                        let response = handler(caller, ($( $names, )*))
                            .map_err(|error| Trap::new(error.to_string()))?;
                        Ok(response)
                    },
                )?;
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
