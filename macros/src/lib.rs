use convert_case::{Case, Casing};
use darling::{FromDeriveInput, FromField, FromMeta, ast::NestedMeta, usage::IdentSet};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, parse_macro_input};

// ---------------------------------------------------------------------------
// Attribute parsing
// ---------------------------------------------------------------------------

#[derive(FromDeriveInput, Debug)]
#[darling(attributes(entschema))]
struct EntSchemaArgs {
    table: String,
}

/// Minimal field info collected by darling.
#[derive(Debug, Clone, FromField)]
struct EntStructField {
    ident: Option<syn::Ident>,
    ty: syn::Type,
}

// ---------------------------------------------------------------------------
// Proc macro entry point
// ---------------------------------------------------------------------------

#[proc_macro_derive(EntSchema, attributes(entschema, edge))]
pub fn derive_ent_schema(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::DeriveInput);

    // --- Parse #[entschema(...)] ---
    let args = match EntSchemaArgs::from_derive_input(&input) {
        Ok(v) => v,
        Err(e) => return e.write_errors().into(),
    };

    // --- Parse struct fields ---
    let named_fields = match &input.data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(f),
            ..
        }) => &f.named,
        _ => {
            return syn::Error::new_spanned(
                &input.ident,
                "EntSchema can only be derived for structs with named fields",
            )
            .to_compile_error()
            .into();
        }
    };
    let fields: Vec<EntStructField> = match named_fields
        .iter()
        .map(|f| EntStructField::from_field(f))
        .collect::<darling::Result<Vec<_>>>()
    {
        Ok(v) => v,
        Err(e) => return e.write_errors().into(),
    };

    // --- Code generation ---
    let name = &input.ident;
    let mod_name = format_ident!("{}", name.to_string().to_case(Case::Snake));
    let mutator_name = format_ident!("{}Mutation", name);
    let table_str = &args.table;

    let field_structs = fields
        .iter()
        .map(|field| field.ent_field_def(name, &mutator_name));

    let mutator_fields = fields.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        let field_struct_name = format_ident!("{}", ident.to_string().to_case(Case::Pascal));
        quote! {
            #ident: resent::mutator::EntMutationFieldState<#mod_name::#field_struct_name>
        }
    });

    // let primary_key_loader_method = gen_primary_key_loader_method(&args.primary_key, &fields);

    let field_assignments = fields.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        quote! {
            #ident: row.get(stringify!(#ident))
        }
    });

    quote! {
        pub mod #mod_name {
            use super::*;

            #(#field_structs)*
        }

        impl resent::Ent for #name {
            const TABLE_NAME: &'static str = #table_str;
        }

        struct #mutator_name {
            ent: #name,
            #(#mutator_fields),*
        }

        impl<'ctx, Ctx: 'ctx + Sync> resent::mutator::EntMutator<'ctx, Ctx, #name>
        for #mutator_name
        where
            #name: resent::privacy::EntPrivacyPolicy<'ctx, Ctx>
        {
            fn get_ent(&self) -> &#name {
                &self.ent
            }
        }

        impl From<sqlx::postgres::PgRow> for #name {
            fn from(row: sqlx::postgres::PgRow) -> Self {
                use sqlx::Row;
                Self {
                    #(#field_assignments),*
                }
            }
        }
    }
    .into()
}

// ---------------------------------------------------------------------------
// Code generation helpers
// ---------------------------------------------------------------------------

impl EntStructField {
    fn struct_name(&self) -> proc_macro2::Ident {
        format_ident!(
            "{}",
            self.ident
                .as_ref()
                .unwrap()
                .to_string()
                .to_case(Case::Pascal)
        )
    }

    fn ent_field_def(
        &self,
        ent_name: &syn::Ident,
        ent_mutator_name: &syn::Ident,
    ) -> proc_macro2::TokenStream {
        let ident = self.ident.as_ref().unwrap();
        let struct_name = self.struct_name();
        let field_name = ident.to_string();
        let field_type = &self.ty;
        quote! {
            pub struct #struct_name;

            impl resent::field::EntField for #struct_name {
                const NAME: &'static str = #field_name;
                type Value = #field_type;
                type Ent = #ent_name;

                fn get_value(ent: &Self::Ent) -> &Self::Value {
                    &ent.#ident
                }
            }

            impl resent::field::EntFieldSetter<#ent_mutator_name> for #struct_name {
                fn set(target: &mut #ent_mutator_name, new_value: Self::Value) {
                    target.#ident = resent::mutator::EntMutationFieldState::Set(Box::new(new_value));
                }
            }
        }
    }
}
