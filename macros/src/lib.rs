use convert_case::{Case, Casing};
use darling::{FromDeriveInput, FromField};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse_macro_input;

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
#[darling(attributes(field))]
struct EntStructField {
    ident: Option<syn::Ident>,
    ty: syn::Type,

    #[darling(default)]
    readonly: bool,

    #[darling(default)]
    primary_key: bool,
}

// ---------------------------------------------------------------------------
// Proc macro entry point
// ---------------------------------------------------------------------------

#[proc_macro_derive(EntSchema, attributes(entschema, edge, field))]
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
        .map(EntStructField::from_field)
        .collect::<darling::Result<Vec<_>>>()
    {
        Ok(v) => v,
        Err(e) => return e.write_errors().into(),
    };

    let primary_keys: Vec<&EntStructField> = fields.iter().filter(|f| f.primary_key).collect();
    if primary_keys.is_empty() {
        return syn::Error::new_spanned(
            &input.ident,
            "At least one field must be marked as primary_key",
        )
        .to_compile_error()
        .into();
    }

    // --- Code generation ---
    let name = &input.ident;
    let mod_name = format_ident!("{}", name.to_string().to_case(Case::Snake));
    let table_str = &args.table;

    let field_structs = fields.iter().map(|field| field.ent_field_def(name));

    let field_assignments = fields.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        quote! {
            #ident: row.get(stringify!(#ident))
        }
    });

    let primary_key = if primary_keys.len() == 1 {
        let primary_key_name = primary_keys[0].struct_name();
        quote! { #mod_name::#primary_key_name }
    } else {
        let names = primary_keys.iter().map(|f| f.struct_name());
        quote! {
            (#( #mod_name::#names ),*)
        }
    };

    quote! {
        pub mod #mod_name {
            use super::*;

            #(#field_structs)*
        }

        impl resent::Ent for #name {
            const TABLE_NAME: &'static str = #table_str;
            type PrimaryKey = #primary_key;
        }

        impl<'a> From<&'a sqlx::postgres::PgRow> for #name {
            fn from(row: &'a sqlx::postgres::PgRow) -> Self {
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

    fn ent_field_def(&self, ent_name: &syn::Ident) -> proc_macro2::TokenStream {
        let ident = self.ident.as_ref().unwrap();
        let struct_name = self.struct_name();
        let field_name = ident.to_string();
        let field_type = &self.ty;
        let visibility = if self.readonly {
            quote! { resent::field::ReadOnly }
        } else {
            quote! { resent::field::ReadWrite }
        };
        quote! {
            pub struct #struct_name;

            impl resent::field::EntField for #struct_name {
                const NAME: &'static str = #field_name;
                type Value = #field_type;
                type Ent = #ent_name;
                type Visibility = #visibility;

                fn get_value(ent: &Self::Ent) -> &Self::Value {
                    &ent.#ident
                }
            }
        }
    }
}
