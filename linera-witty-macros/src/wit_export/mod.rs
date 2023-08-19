mod function_information;

use self::function_information::FunctionInformation;
use proc_macro2::{Span, TokenStream};
use proc_macro_error::abort;
use quote::{quote, ToTokens};
use syn::{
    parse_quote, spanned::Spanned, FnArg, Ident, ImplItem, ImplItemMethod, ItemImpl, LitStr, Pat,
    PatIdent, PatType, Path, PathArguments, PathSegment, Token, TraitBoundModifier,
    TraitItemMethod, Type, TypeParamBound, TypePath, Visibility,
};

pub fn generate(implementation: &ItemImpl, namespace: &LitStr) -> TokenStream {
    WitExportGenerator::new(implementation, namespace).generate()
}

pub struct WitExportGenerator<'input> {
    type_name: &'input Ident,
    implementation: &'input ItemImpl,
    functions: Vec<FunctionInformation<'input>>,
    namespace: &'input LitStr,
}

impl<'input> WitExportGenerator<'input> {
    pub fn new(implementation: &'input ItemImpl, namespace: &'input LitStr) -> Self {
        let type_name = type_name(implementation);
        let functions = implementation
            .items
            .iter()
            .map(FunctionInformation::from)
            .collect();

        WitExportGenerator {
            type_name,
            implementation,
            functions,
            namespace,
        }
    }

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

pub fn type_name(implementation: &ItemImpl) -> &Ident {
    let Type::Path(TypePath { qself: None, path: path_name }) = &*implementation.self_ty else {
        abort!(
            implementation.self_ty,
            "`#[wit_export]` must be used on `impl` blocks of non-generic types",
        );
    };

    path_name.get_ident().unwrap_or_else(|| {
        abort!(
            implementation.self_ty,
            "`#[wit_export]` must be used on `impl` blocks of non-generic types",
        );
    })
}

fn generate_reentrant_trait_functions(implementation: &ItemImpl) -> Vec<TokenStream> {
    let generic_type_parameter = Ident::new("Runtime", Span::call_site());

    reentrant_functions(implementation)
        .cloned()
        .map(|function| specialize_reentrant_function(function, &generic_type_parameter, true))
        .collect()
}

fn reentrant_functions(implementation: &ItemImpl) -> impl Iterator<Item = &ImplItemMethod> + Clone {
    functions(implementation).filter(is_reentrant_function)
}

fn functions(implementation: &ItemImpl) -> impl Iterator<Item = &ImplItemMethod> + Clone {
    implementation.items.iter().map(|item| match item {
        ImplItem::Method(function) => function,
        ImplItem::Const(const_item) => abort!(
            const_item.ident,
            "Const items are not supported in exported types"
        ),
        ImplItem::Type(type_item) => abort!(
            type_item.ident,
            "Type items are not supported in exported types"
        ),
        ImplItem::Macro(macro_item) => abort!(
            macro_item.mac.path,
            "Macro items are not supported in exported types"
        ),
        _ => {
            abort!(item, "Only function items are supported in exported types")
        }
    })
}

fn is_reentrant_function(function: &&ImplItemMethod) -> bool {
    function
        .sig
        .inputs
        .first()
        .map(|first_input| match first_input {
            FnArg::Receiver(_) => false,
            FnArg::Typed(PatType { ty, .. }) => match &**ty {
                Type::ImplTrait(impl_trait) => {
                    !impl_trait.bounds.is_empty()
                        && is_caller_impl_trait(
                            impl_trait
                                .bounds
                                .first()
                                .expect("Missing element from list of size 1"),
                        )
                }
                _ => false,
            },
        })
        .unwrap_or(false)
}

fn is_caller_impl_trait(bound: &TypeParamBound) -> bool {
    let TypeParamBound::Trait(trait_bound) = bound
        else { return false; };

    trait_bound.paren_token.is_none()
        && matches!(trait_bound.modifier, TraitBoundModifier::None)
        && trait_bound.lifetimes.is_none()
        && is_caller_path(&trait_bound.path)
}

fn is_caller_path(path: &Path) -> bool {
    let mut segments = path.segments.iter();

    let path_is_correct = if path.segments.len() == 1 {
        is_path_segment(
            segments.next().expect("Missing path segment"),
            "Caller",
            true,
        )
    } else if path.segments.len() == 2 {
        is_path_segment(
            segments.next().expect("Missing path segment"),
            "witty",
            false,
        ) && is_path_segment(
            segments.next().expect("Missing path segment"),
            "Caller",
            true,
        )
    } else {
        false
    };

    path_is_correct && path.leading_colon.is_none()
}

fn is_path_segment(
    segment: &PathSegment,
    expected_identifier: &str,
    with_type_parameters: bool,
) -> bool {
    let arguments_are_correct = if with_type_parameters {
        matches!(segment.arguments, PathArguments::AngleBracketed(_))
    } else {
        matches!(segment.arguments, PathArguments::None)
    };

    segment.ident == expected_identifier && arguments_are_correct
}

fn specialize_reentrant_function(
    mut function: ImplItemMethod,
    new_caller_type: impl ToTokens,
    for_trait: bool,
) -> TokenStream {
    let Some(FnArg::Typed(PatType { pat, ty, .. })) = function.sig.inputs.first_mut()
        else { unreachable!("Attempt to specialize a non-reentrant function") };

    *ty = parse_quote!(#new_caller_type);
    function.vis = Visibility::Inherited;

    if for_trait {
        let Pat::Ident(PatIdent { mutability, .. }) = &mut **pat
            else { abort!(pat.span(), "Caller parameter must be a name") };

        *mutability = None;

        let span = function.sig.span();

        TraitItemMethod {
            attrs: vec![],
            sig: function.sig,
            default: None,
            semi_token: Some(Token![;](span)),
        }
        .to_token_stream()
    } else {
        function.to_token_stream()
    }
}

fn reentrancy_constraints(function: &ImplItemMethod) -> impl Iterator<Item = &TypeParamBound> + '_ {
    let Some(FnArg::Typed(PatType { ty, .. })) = function.sig.inputs.first()
        else { unreachable!("Attempt to get data type parameter of a non-reentrant function") };
    let Type::ImplTrait(impl_trait) = &**ty
        else { unreachable!("Attempt to get data type parameter of a non-reentrant function") };

    impl_trait.bounds.iter()
}
