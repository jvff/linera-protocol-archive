// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Generation of code to export host functions to a Wasm guest instance.

#![cfg(any(feature = "mock-instance", feature = "wasmer", feature = "wasmtime"))]

mod function_information;

pub(crate) use self::function_information::{ok_type_inside_result, FunctionInformation};

use super::wit_interface;
use crate::util::AttributeParameters;
use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use std::collections::HashMap;
use syn::{
    punctuated::Punctuated, token::Paren, AngleBracketedGenericArguments, AssocType,
    GenericArgument, Generics, Ident, ItemImpl, LitStr, PathArguments, PathSegment, PredicateType,
    Token, TraitBound, TraitBoundModifier, Type, TypeParam, TypeParamBound, TypePath, TypeTuple,
    WhereClause, WherePredicate,
};

/// Returns the code generated for exporting host functions to guest Wasm instances.
///
/// The generated code implements the `linera_witty::ExportTo` trait for the Wasm runtimes enabled
/// through feature flags. The trait implementation exports the host functions in the input `impl`
/// block to a provided Wasm guest instance.
pub fn generate(implementation: &ItemImpl, parameters: AttributeParameters) -> TokenStream {
    WitExportGenerator::new(implementation, parameters).generate()
}

/// A helper type for generation of the code to export host functions to Wasm guest instances.
///
/// Code generating is done in two phases. First the necessary pieces are collected and stored in
/// this type. Then, they are used to generate the final code.
pub struct WitExportGenerator<'input> {
    parameters: AttributeParameters,
    namespace: LitStr,
    type_name: &'input Ident,
    caller_type_parameter: CallerTypeParameter<'input>,
    implementation: &'input ItemImpl,
    functions: Vec<FunctionInformation<'input>>,
}

impl<'input> WitExportGenerator<'input> {
    /// Collects the pieces necessary for code generation from the inputs.
    pub fn new(implementation: &'input ItemImpl, parameters: AttributeParameters) -> Self {
        let type_name = type_name(implementation);
        let namespace = parameters.namespace(type_name);
        let caller_type_parameter = CallerTypeParameter::new(&implementation.generics);
        let functions = implementation
            .items
            .iter()
            .map(|item| FunctionInformation::from_item(item, caller_type_parameter.caller()))
            .collect();

        WitExportGenerator {
            parameters,
            namespace,
            type_name,
            caller_type_parameter,
            implementation,
            functions,
        }
    }

    /// Consumes the collected pieces to generate the final code.
    pub fn generate(mut self) -> TokenStream {
        let implementation = self.implementation;
        let wasmer = self.generate_for_wasmer();
        let wasmtime = self.generate_for_wasmtime();
        let mock_instance = self.generate_for_mock_instance();
        let wit_interface = self.generate_wit_interface();

        quote! {
            #implementation
            #wasmer
            #wasmtime
            #mock_instance
            #wit_interface
        }
    }

    /// Generates the code to export functions using the Wasmer runtime.
    fn generate_for_wasmer(&mut self) -> Option<TokenStream> {
        #[cfg(feature = "wasmer")]
        {
            let user_data_type = self.user_data_type();
            let export_target = quote! { linera_witty::wasmer::InstanceBuilder<#user_data_type> };
            let target_caller_type = quote! {
                linera_witty::wasmer::FunctionEnvMut<
                    '_,
                    linera_witty::wasmer::InstanceSlot<#user_data_type>,
                >
            };
            let exported_functions = self.functions.iter().map(|function| {
                function.generate_for_wasmer(&self.namespace, self.type_name, &target_caller_type)
            });

            Some(self.generate_for(export_target, &target_caller_type, exported_functions))
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
            let user_data_type = self.user_data_type();
            let export_target = quote! { linera_witty::wasmtime::Linker<#user_data_type> };
            let target_caller_type = quote! { linera_witty::wasmtime::Caller<'_, #user_data_type> };
            let exported_functions = self.functions.iter().map(|function| {
                function.generate_for_wasmtime(&self.namespace, self.type_name, &target_caller_type)
            });

            Some(self.generate_for(export_target, &target_caller_type, exported_functions))
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
            let user_data_type = self.user_data_type();
            let export_target = quote! { linera_witty::MockInstance<#user_data_type> };
            let target_caller_type = quote! { linera_witty::MockInstance<#user_data_type> };
            let exported_functions = self.functions.iter().map(|function| {
                function.generate_for_mock_instance(
                    &self.namespace,
                    self.type_name,
                    &target_caller_type,
                )
            });

            Some(self.generate_for(export_target, &target_caller_type, exported_functions))
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
        target_caller_type: &TokenStream,
        exported_functions: impl Iterator<Item = TokenStream>,
    ) -> TokenStream {
        let type_name = &self.type_name;
        let caller_type_parameter = self
            .caller_type_parameter
            .caller()
            .map(|_| quote! { <#target_caller_type> });

        quote! {
            impl linera_witty::ExportTo<#export_target> for #type_name #caller_type_parameter {
                fn export_to(
                    target: &mut #export_target,
                ) -> Result<(), linera_witty::RuntimeError> {
                    #( #exported_functions )*
                    Ok(())
                }
            }
        }
    }

    /// Returns the type to use for the custom user data.
    fn user_data_type(&self) -> Type {
        self.caller_type_parameter
            .user_data()
            .cloned()
            .unwrap_or_else(|| {
                // Unit type
                Type::Tuple(TypeTuple {
                    paren_token: Paren::default(),
                    elems: Punctuated::new(),
                })
            })
    }

    /// Generates the implementation of `WitInterface` for the type.
    fn generate_wit_interface(&self) -> TokenStream {
        let type_name = self.type_name;

        let wit_interface_implementation = wit_interface::generate(
            self.parameters.package_name(),
            self.parameters.interface_name(type_name),
            &self.functions,
        );

        let (impl_generics, type_generics, where_clause) =
            self.implementation.generics.split_for_impl();

        quote! {
            impl #impl_generics linera_witty::wit_generation::WitInterface
                for #type_name #type_generics
            #where_clause
            {
                #wit_interface_implementation
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

/// Information on the  generic type parameter to use for the caller parameter, if present.
#[derive(Clone, Copy, Debug)]
enum CallerTypeParameter<'input> {
    NotPresent,
    WithoutUserData(&'input Ident),
    WithUserData {
        caller: &'input Ident,
        user_data: &'input Type,
    },
}

impl<'input> CallerTypeParameter<'input> {
    /// Parses a type's [`Generics`] to determine if a caller type parameter should be used.
    pub fn new(generics: &'input Generics) -> Self {
        let where_bounds = Self::parse_bounds_from_where_clause(generics.where_clause.as_ref());

        generics
            .type_params()
            .filter_map(|parameter| Self::try_from_parameter(parameter, &where_bounds))
            .next()
            .unwrap_or(CallerTypeParameter::NotPresent)
    }

    fn parse_bounds_from_where_clause(
        where_clause: Option<&'input WhereClause>,
    ) -> HashMap<&'input Ident, Vec<&'input TypeParamBound>> {
        where_clause
            .into_iter()
            .flat_map(|where_clause| where_clause.predicates.iter())
            .filter_map(|predicate| match predicate {
                WherePredicate::Type(predicate) => Self::extract_predicate_bounds(predicate),
                _ => None,
            })
            .collect()
    }

    fn extract_predicate_bounds(
        predicate: &'input PredicateType,
    ) -> Option<(&'input Ident, Vec<&'input TypeParamBound>)> {
        let target_identifier = Self::extract_identifier(&predicate.bounded_ty)?;

        Some((target_identifier, predicate.bounds.iter().collect()))
    }

    fn extract_identifier(candidate_type: &'input Type) -> Option<&'input Ident> {
        let Type::Path(TypePath { qself: None, path }) = candidate_type else {
            return None;
        };

        if path.leading_colon.is_some() || path.segments.len() != 1 {
            return None;
        }

        let segment = path.segments.first()?;

        if !matches!(&segment.arguments, PathArguments::None) {
            return None;
        }

        Some(&segment.ident)
    }

    fn try_from_parameter(
        parameter: &'input TypeParam,
        where_bounds: &HashMap<&'input Ident, Vec<&'input TypeParamBound>>,
    ) -> Option<Self> {
        let caller = &parameter.ident;

        let bounds = where_bounds
            .get(caller)
            .into_iter()
            .flatten()
            .copied()
            .chain(parameter.bounds.iter());

        let instance_bound_path_segment = bounds
            .filter_map(Self::extract_trait_bound_path)
            .filter_map(Self::extract_instance_bound_path_segment)
            .next()?;

        let maybe_user_data =
            Self::extract_instance_bound_arguments(&instance_bound_path_segment.arguments)
                .and_then(Self::extract_instance_bound_user_data);

        match maybe_user_data {
            Some(user_data) => Some(CallerTypeParameter::WithUserData { caller, user_data }),
            None => Some(CallerTypeParameter::WithoutUserData(caller)),
        }
    }

    /// Extracts the path from a trait `bound`.
    fn extract_trait_bound_path(
        bound: &'input TypeParamBound,
    ) -> Option<impl Iterator<Item = &'input PathSegment> + Clone + 'input> {
        match bound {
            TypeParamBound::Trait(TraitBound {
                paren_token: None,
                modifier: TraitBoundModifier::None,
                lifetimes: None,
                path,
            }) => Some(path.segments.iter()),
            _ => None,
        }
    }

    fn extract_instance_bound_path_segment(
        segments: impl Iterator<Item = &'input PathSegment> + Clone,
    ) -> Option<&'input PathSegment> {
        Self::extract_aliased_instance_bound_path_segment(segments.clone())
            .or_else(|| Self::extract_direct_instance_bound_path_segment(segments))
    }

    /// Extracts the generic arguments inside the caller parameter path's `segments`.
    fn extract_aliased_instance_bound_path_segment(
        mut segments: impl Iterator<Item = &'input PathSegment>,
    ) -> Option<&'input PathSegment> {
        let segment = segments.next()?;

        if segment.ident.to_string().starts_with("InstanceFor") && segments.next().is_none() {
            Some(segment)
        } else {
            None
        }
    }

    /// Extracts the generic arguments inside the caller parameter path's `segments`.
    fn extract_direct_instance_bound_path_segment(
        segments: impl Iterator<Item = &'input PathSegment>,
    ) -> Option<&'input PathSegment> {
        let mut segments = segments.peekable();

        if matches!(
            segments.peek(),
            Some(PathSegment { ident, arguments: PathArguments::None })
                if ident == "linera_witty",
        ) {
            segments.next();
        }

        let segment = segments.next()?;

        if segment.ident == "Instance" && segments.next().is_none() {
            Some(segment)
        } else {
            None
        }
    }

    fn extract_instance_bound_arguments(
        arguments: &'input PathArguments,
    ) -> Option<&'input Punctuated<GenericArgument, Token![,]>> {
        match arguments {
            PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                colon2_token: None,
                args,
                ..
            }) => Some(args),
            _ => None,
        }
    }

    /// Extracts the custom user data [`Type`] from the caller bound's generic `arguments`.
    fn extract_instance_bound_user_data(
        arguments: &'input Punctuated<GenericArgument, Token![,]>,
    ) -> Option<&'input Type> {
        if arguments.len() != 1 {
            abort!(
                arguments,
                "Caller type parameter should have a user data type. \
                E.g. `Caller: linera_witty::Instance<UserData = CustomData>`"
            );
        }

        match arguments
            .iter()
            .next()
            .expect("Missing argument in arguments list")
        {
            GenericArgument::AssocType(AssocType {
                ident,
                generics: None,
                ty: user_data,
                ..
            }) if ident == "UserData" => Some(user_data),
            _ => abort!(
                arguments,
                "Caller type parameter should have a user data type. \
                E.g. `Caller: linera_witty::Instance<UserData = CustomData>`"
            ),
        }
    }

    /// Returns the [`Ident`]ifier of the generic type parameter used for the caller.
    pub fn caller(&self) -> Option<&'input Ident> {
        match self {
            CallerTypeParameter::NotPresent => None,
            CallerTypeParameter::WithoutUserData(caller) => Some(caller),
            CallerTypeParameter::WithUserData { caller, .. } => Some(caller),
        }
    }

    /// Returns the type used for custom user data, if there is a caller type parameter.
    pub fn user_data(&self) -> Option<&'input Type> {
        match self {
            CallerTypeParameter::NotPresent => None,
            CallerTypeParameter::WithoutUserData(_) => None,
            CallerTypeParameter::WithUserData { user_data, .. } => Some(user_data),
        }
    }
}
