// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Generation of code to export host functions to a Wasm guest instance.

#![cfg(any(feature = "mock-instance", feature = "wasmer", feature = "wasmtime"))]

mod function_information;

use self::function_information::FunctionInformation;
use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use syn::{Ident, ItemImpl, LitStr, Type, TypePath};

/// Returns the code generated for exporting host functions to guest Wasm instances.
///
/// The generated code implements the `linera_witty::ExportTo` trait for the Wasm runtimes enabled
/// through feature flags. The trait implementation exports the host functions in the input `impl`
/// block to a provided Wasm guest instance.
pub fn generate(implementation: &ItemImpl, namespace: &LitStr) -> TokenStream {
    WitExportGenerator::new(implementation, namespace).generate()
}

/// A helper type for generation of the code to export host functions to Wasm guest instances.
///
/// Code generating is done in two phases. First the necessary pieces are collected and stored in
/// this type. Then, they are used to generate the final code.
pub struct WitExportGenerator<'input> {
    type_name: &'input Ident,
    implementation: &'input ItemImpl,
    functions: Vec<FunctionInformation<'input>>,
    namespace: &'input LitStr,
}

impl<'input> WitExportGenerator<'input> {
    /// Collects the pieces necessary for code generation from the inputs.
    pub fn new(implementation: &'input ItemImpl, namespace: &'input LitStr) -> Self {
        let type_name = type_name(implementation);
        let functions = implementation
            .items
            .iter()
            .map(FunctionInformation::from_item)
            .collect();

        WitExportGenerator {
            type_name,
            implementation,
            functions,
            namespace,
        }
    }

    /// Consumes the collected pieces to generate the final code.
    pub fn generate(mut self) -> TokenStream {
        let implementation = self.implementation;
        let wasmer = self.generate_for_wasmer();
        let wasmtime = self.generate_for_wasmtime();
        let mock_instance = self.generate_for_mock_instance();

        quote! {
            #implementation
            #wasmer
            #wasmtime
            #mock_instance
        }
    }

    /// Generates the code to export functions using the Wasmer runtime.
    fn generate_for_wasmer(&mut self) -> Option<TokenStream> {
        #[cfg(feature = "wasmer")]
        {
            let export_target = quote! { linera_witty::wasmer::InstanceBuilder };
            let exported_functions = self
                .functions
                .iter()
                .map(|function| function.generate_for_wasmer(self.namespace));

            Some(self.generate_for(export_target, exported_functions))
        }
        #[cfg(not(feature = "wasmer"))]
        {
            None
        }
    }

    /// Generates the code to export functions using the Wasmtime runtime.
    fn generate_for_wasmtime(&mut self) -> Option<TokenStream> {
        #[cfg(feature = "wasmtime")]
        {
            let export_target = quote! { linera_witty::wasmtime::Linker<()> };
            let exported_functions = self
                .functions
                .iter()
                .map(|function| function.generate_for_wasmtime(self.namespace));

            Some(self.generate_for(export_target, exported_functions))
        }
        #[cfg(not(feature = "wasmtime"))]
        {
            None
        }
    }

    /// Generates the code to export functions to a mock instance for testing.
    fn generate_for_mock_instance(&mut self) -> Option<TokenStream> {
        #[cfg(feature = "mock-instance")]
        {
            let export_target = quote! { linera_witty::MockInstance };
            let exported_functions = self
                .functions
                .iter()
                .map(|function| function.generate_for_mock_instance(self.namespace));

            Some(self.generate_for(export_target, exported_functions))
        }
        #[cfg(not(feature = "mock-instance"))]
        {
            None
        }
    }

    /// Generates the implementation of `ExportTo` for the `export_target` including the
    /// `exported_functions`.
    fn generate_for(
        &self,
        export_target: TokenStream,
        exported_functions: impl Iterator<Item = TokenStream>,
    ) -> TokenStream {
        let type_name = &self.type_name;

        quote! {
            impl linera_witty::ExportTo<#export_target> for #type_name {
                fn export_to(
                    target: &mut #export_target,
                ) -> Result<(), linera_witty::RuntimeError> {
                    #( #exported_functions )*
                    Ok(())
                }
            }
        }
    }
}

/// Returns the type name of the type the `impl` block is for.
pub fn type_name(implementation: &ItemImpl) -> &Ident {
    let Type::Path(TypePath {
        qself: None,
        path: path_name,
    }) = &*implementation.self_ty
    else {
        abort!(
            implementation.self_ty,
            "`#[wit_export]` must be used on `impl` blocks",
        );
    };

    &path_name
        .segments
        .last()
        .unwrap_or_else(|| {
            abort!(implementation.self_ty, "Missing type name identifier",);
        })
        .ident
}
