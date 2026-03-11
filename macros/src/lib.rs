use convert_case::{Case, Casing};
use darling::{FromDeriveInput, FromField, FromMeta, ast::NestedMeta, usage::IdentSet};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, parse_macro_input};

// ---------------------------------------------------------------------------
// Attribute parsing
// ---------------------------------------------------------------------------

#[derive(FromMeta, Debug)]
#[darling(from_expr = |expr| Ok(EntPrimaryKey::from(expr)))]
enum EntPrimaryKey {
    // #[darling(skip)]
    Single(syn::Path),
    // #[darling(skip)]
    Composite(darling::util::PathList),
}

impl From<&syn::Expr> for EntPrimaryKey {
    fn from(expr: &syn::Expr) -> Self {
        match expr {
            syn::Expr::Path(p) => EntPrimaryKey::Single(p.path.clone()),
            syn::Expr::Array(a) => {
                let paths = a
                    .elems
                    .iter()
                    .filter_map(|elem| {
                        if let syn::Expr::Path(p) = elem {
                            Some(p.path.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<syn::Path>>();

                if paths.len() != a.elems.len() {
                    panic!("primary_key array must contain only paths");
                }

                if paths.len() == 1 {
                    return EntPrimaryKey::Single(paths.into_iter().next().unwrap());
                }

                EntPrimaryKey::Composite(paths.into())
            }
            _ => panic!("primary_key must be a path or an array of paths"),
        }
    }
}

#[derive(FromDeriveInput, Debug)]
#[darling(attributes(entschema))]
struct EntSchemaArgs {
    table: String,
    // Support primary_key = id or primary_key = [id, other_id] (for composite keys)
    primary_key: Option<EntPrimaryKey>,
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
        mod #mod_name {
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
    fn filter_method_name(&self) -> proc_macro2::Ident {
        format_ident!("where_{}", self.ident.as_ref().unwrap())
    }

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

    fn ent_field_module_name(ent_name: &syn::Ident) -> proc_macro2::TokenStream {
        let ent_module_name = format_ident!("{}", ent_name.to_string().to_case(Case::Snake));
        quote! { #ent_module_name }
    }
}

// /// Generates a loader method for the primary key, e.g. `load(ctx, id) -> Ent`
// fn gen_primary_key_loader_method(
//     primary_key: &Option<EntPrimaryKey>,
//     fields: &[EntStructField],
// ) -> proc_macro2::TokenStream {
//     if let Some(pk) = primary_key {
//         match pk {
//             EntPrimaryKey::Single(field) => {
//                 let field_name =
//                     format_ident!("{}", field.segments.last().unwrap().ident.to_string());
//                 let filter_name = format_ident!("where_{}", field_name);
//                 let field_type = fields
//                     .iter()
//                     .find(|f| f.ident.as_ref().unwrap() == &field_name)
//                     .unwrap()
//                     .ty
//                     .clone();
//                 return quote! {
//                     pub async fn load<'ctx, Ctx: 'ctx + Sync>(
//                         ctx: &'ctx resent::query::QueryContext<Ctx>,
//                         #field_name: #field_type
//                     ) -> Result<Self, resent::query::EntLoadOnlyError>
//                     where
//                         Self: resent::privacy::EntPrivacyPolicy<'ctx, Ctx>
//                     {
//                         use resent::{Ent, query::EntQuery};
//                         Self::query(ctx).#filter_name(resent::predicate::QueryPredicate::Equals(#field_name)).load_only().await
//                     }
//                 };
//             }
//             _ => {}
//         };
//     }

//     quote! {}
// }

/// Converts an entity type path (e.g. `EntBar`) into a query method name (e.g. `query_bar`).
fn query_fn_name(path: &syn::Path) -> proc_macro2::Ident {
    format_ident!(
        "query_{}",
        path.segments
            .last()
            .unwrap()
            .ident
            .to_string()
            .to_case(Case::Snake)
            .trim_start_matches("ent_")
    )
}
