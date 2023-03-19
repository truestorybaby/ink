// Copyright 2018-2022 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::GenerateCode;
use derive_more::From;
use ir::ChainExtensionMethod;
use proc_macro2::TokenStream as TokenStream2;
use quote::{
    format_ident,
    quote_spanned,
};
use syn::spanned::Spanned;

/// Generator to create an ink! chain extension.
#[derive(From)]
pub struct ChainExtension<'a> {
    extension: &'a ir::ChainExtension,
}

impl ChainExtension<'_> {
    fn generate_for_instance_method(
        method: &ChainExtensionMethod,
        error_code: &syn::Type,
    ) -> TokenStream2 {
        let span = method.span();
        let attrs = method.attrs();
        let ident = method.ident();
        let func_id = method.id().into_u32();
        let sig = method.sig();
        let inputs = &sig.inputs;
        let input_bindings = method.inputs().map(|pat_type| &pat_type.pat);
        let input_types = method.inputs().map(|pat_type| &pat_type.ty);

        // Mostly tuple-based type representation of the inputs:
        // Special case for when there is exactly one input type.
        // - 0 inputs          -> ()
        // - 1 input T         -> T
        // - n inputs A, B. .. -> (A, B, ..)
        let compound_input_type = match inputs.len() {
            0 => quote_spanned!(span=> ()),
            1 => quote_spanned!(span=> #( #input_types )* ),
            _n => quote_spanned!(span=> ( #( #input_types ),* ) ),
        };

        // Mostly tuple-based value representation of the inputs:
        // Special case for when there is exactly one input value.
        // - 0 inputs          -> ()
        // - 1 input a         -> a
        // - n inputs a, b. .. -> (a, b, ..)
        let compound_input_bindings = match inputs.len() {
            0 => quote_spanned!(span=> ()),
            1 => quote_spanned!(span=> #( #input_bindings )* ),
            _n => quote_spanned!(span=> ( #( #input_bindings ),* ) ),
        };

        let output = &sig.output;
        let output_type = match output {
            syn::ReturnType::Default => quote_spanned!(output.span()=> ()),
            syn::ReturnType::Type(_arrow, ty) => {
                quote_spanned!(output.span()=> #ty)
            }
        };

        let handle_status = method.handle_status();

        let handle_status_token = if handle_status {
            quote_spanned!(span=>
                true
            )
        } else {
            quote_spanned!(span=>
                false
            )
        };

        let error_code_handling = if handle_status {
            quote_spanned!(span=>
                .handle_error_code::<#error_code>()
            )
        } else {
            quote_spanned!(span=>
                .ignore_error_code()
            )
        };

        let return_type = quote_spanned!(span =>
            <::ink::ValueReturned as ::ink::Output<{ ::ink::is_result_type!(#output_type) }, #handle_status_token, #output_type, #error_code>>::ReturnType
        );

        // we only need to check if handle status is set to true to enable this type bound
        let where_output_impls_from_error_code = Some(quote_spanned!(span=>
            <#return_type as ::ink::IsResultType>::Err: ::core::convert::From<#error_code>,
        )).filter(|_|  handle_status);

        quote_spanned!(span=>
                #( #attrs )*
                #[inline]
                pub fn #ident(self, #inputs) -> #return_type
                where
                    #where_output_impls_from_error_code
                {
                    ::ink::env::chain_extension::ChainExtensionMethod::build(#func_id)
                    .input::<#compound_input_type>()
                    .output::<#output_type, {::ink::is_result_type!(#output_type)}>()
                    #error_code_handling
                    .call(&#compound_input_bindings)
                }
        )
    }
}

impl GenerateCode for ChainExtension<'_> {
    fn generate_code(&self) -> TokenStream2 {
        let span = self.extension.span();
        let attrs = self.extension.attrs();
        let ident = self.extension.ident();
        let error_code = self.extension.error_code();
        let instance_methods = self
            .extension
            .iter_methods()
            .map(|method| Self::generate_for_instance_method(method, error_code));
        let instance_ident = format_ident!("__ink_{}Instance", ident);
        quote_spanned!(span =>
            #(#attrs)*
            pub enum #ident {}

            const _: () = {
                #[allow(non_camel_case_types)]
                struct __ink_Private;
                #[allow(non_camel_case_types)]
                pub struct #instance_ident {
                    __ink_private: __ink_Private
                }

                impl #instance_ident {
                    #( #instance_methods )*
                }

                impl ::ink::ChainExtensionInstance for #ident {
                    type Instance = #instance_ident;

                    fn instantiate() -> Self::Instance {
                        Self::Instance { __ink_private: __ink_Private }
                    }
                }
            };
        )
    }
}
