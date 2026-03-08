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
                let paths = a.elems.iter().filter_map(|elem| {
                    if let syn::Expr::Path(p) = elem {
                        Some(p.path.clone())
                    } else {
                        None
                    }
                }).collect::<Vec<syn::Path>>();

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
    ctx: syn::Path,

    // Support primary_key = id or primary_key = [id, other_id] (for composite keys)
    primary_key: Option<EntPrimaryKey>,
}

/// Parses either:
///   `#[edge(to = BarEnt, on = "id", from = "bar_id")]`  — this struct holds a FK to BarEnt
///   `#[edge(from = FooEnt, on = "bar_id", to = "id")]`  — FooEnt holds a FK to this struct
///
/// In the `to` form: `to` is a Path (the remote type), `from` is a String (remote PK field).
/// In the `from` form: `from` is a Path (the referencing type), `to` is a String (remote FK field).
#[derive(Debug, Clone)]
enum EntSchemaEdge {
    To {
        entity: syn::Path,
        on: String,
        from_field: String,
    },
    From {
        entity: syn::Path,
        on: String,
        to_field: String,
    },
}

impl FromMeta for EntSchemaEdge {
    fn from_list(items: &[NestedMeta]) -> darling::Result<Self> {
        let mut to_path: Option<syn::Path> = None;
        let mut from_path: Option<syn::Path> = None;
        let mut on: Option<String> = None;
        let mut to_str: Option<String> = None;
        let mut from_str: Option<String> = None;

        for item in items {
            let NestedMeta::Meta(syn::Meta::NameValue(nv)) = item else {
                return Err(darling::Error::custom("unexpected edge attribute item"));
            };
            let key = nv
                .path
                .get_ident()
                .map(|i| i.to_string())
                .unwrap_or_default();
            match key.as_str() {
                "to" => match &nv.value {
                    syn::Expr::Path(p) => to_path = Some(p.path.clone()),
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) => to_str = Some(s.value()),
                    _ => {
                        return Err(darling::Error::custom(
                            "edge `to` must be an ident (entity type) or string (field name)",
                        ));
                    }
                },
                "from" => match &nv.value {
                    syn::Expr::Path(p) => from_path = Some(p.path.clone()),
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) => from_str = Some(s.value()),
                    _ => {
                        return Err(darling::Error::custom(
                            "edge `from` must be an ident (entity type) or string (field name)",
                        ));
                    }
                },
                "on" => match &nv.value {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) => on = Some(s.value()),
                    _ => return Err(darling::Error::custom("edge `on` must be a string")),
                },
                other => return Err(darling::Error::unknown_field(other)),
            }
        }

        let on = on.ok_or_else(|| darling::Error::missing_field("on"))?;

        if let Some(entity) = to_path {
            let from_field = from_str.ok_or_else(|| darling::Error::missing_field("from"))?;
            Ok(EntSchemaEdge::To {
                entity,
                on,
                from_field,
            })
        } else if let Some(entity) = from_path {
            let to_field = to_str.ok_or_else(|| darling::Error::missing_field("to"))?;
            Ok(EntSchemaEdge::From {
                entity,
                on,
                to_field,
            })
        } else {
            Err(darling::Error::custom(
                "edge must specify `to = EntityType` or `from = EntityType`",
            ))
        }
    }
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

    // --- Parse #[edge(...)] attributes ---
    let mut edges: Vec<EntSchemaEdge> = Vec::new();
    for attr in input.attrs.iter().filter(|a| a.path().is_ident("edge")) {
        match EntSchemaEdge::from_meta(&attr.meta) {
            Ok(e) => edges.push(e),
            Err(e) => return e.write_errors().into(),
        }
    }

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
    let query_trait_name = format_ident!("{}Query", name);
    let mutator_name = format_ident!("{}Mutation", name);
    let table_str = &args.table;
    let ctx_type = &args.ctx;

    let field_structs = gen_field_structs(&fields, ctx_type, name, &mutator_name);

    let mutator_fields = fields.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        let field_struct_name = format_ident!("{}", ident.to_string().to_case(Case::Pascal));
        quote! {
            #ident: resent::mutator::EntMutationFieldState<'ctx, #ctx_type, #name, #mod_name::fields::#field_struct_name>
        }
    });

    let edge_query_methods = gen_edge_query_methods(&edges, ctx_type);
    let primary_key_loader_method = gen_primary_key_loader_method(&args.primary_key, &fields, ctx_type);
    
    let (field_filter_trait_methods, field_filter_impl_methods) =
        gen_field_filter_methods(&fields, &mod_name, ctx_type, name);
    let (edge_ent_query_trait_methods, edge_ent_query_impl_methods) =
        gen_edge_ent_query_methods(&edges, ctx_type);
    
    let field_assignments = fields.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        quote! {
            #ident: row.get(stringify!(#ident))
        }
    });

    quote! {
        mod #mod_name {
            pub mod fields {
                use super::super::*;

                #(#field_structs)*
            }
        }

        impl #name {
            #(#edge_query_methods)*
            #primary_key_loader_method
        }

        impl<'ctx> resent::Ent<'ctx, #ctx_type> for #name {
            const TABLE_NAME: &'static str = #table_str;
        }

        struct #mutator_name<'ctx> {
            ent: #name,
            #(#mutator_fields),*
        }

        impl<'ctx> resent::mutator::EntMutator<'ctx, #ctx_type, #name> for #mutator_name<'ctx> {
            fn get_ent(&self) -> &#name {
                &self.ent
            }
        }

        pub trait #query_trait_name<'ctx> {
            #(#field_filter_trait_methods)*
            #(#edge_ent_query_trait_methods)*
        }

        impl<'ctx> #query_trait_name<'ctx> for resent::query::EntQuery<'ctx, #ctx_type, #name> {
            #(#field_filter_impl_methods)*
            #(#edge_ent_query_impl_methods)*
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

/// EntField impls for each struct field, e.g. `struct Id; impl EntField for Id { ... }`
fn gen_field_structs(
    fields: &[EntStructField],
    ctx_type: &syn::Path,
    name: &syn::Ident,
    mutator_name: &syn::Ident,
) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().unwrap();
            let struct_name = format_ident!("{}", ident.to_string().to_case(Case::Pascal));
            let field_name = ident.to_string();
            let field_type = &field.ty;

            quote! {
                pub struct #struct_name;

                impl<'ctx> resent::field::EntField<'ctx, #ctx_type, #name> for #struct_name {
                    const NAME: &'static str = #field_name;
                    type Value = #field_type;
                }

                impl<'ctx> resent::field::EntFieldSetter<'ctx, #ctx_type, #name, #mutator_name<'ctx>> for #struct_name {
                    fn set(target: &mut #mutator_name<'ctx>, new_value: Self::Value) {
                        target.#ident = resent::mutator::EntMutationFieldState::Set(Box::new(new_value));
                    }
                }
            }
        })
        .collect()
}

/// Generates filter methods for each field, e.g. `where_id(self, predicate) -> EntQuery<...>`
fn gen_field_filter_methods(
    fields: &[EntStructField],
    mod_name: &proc_macro2::Ident,
    ctx_type: &syn::Path,
    name: &syn::Ident,
) -> (Vec<proc_macro2::TokenStream>, Vec<proc_macro2::TokenStream>) {
    fields
        .iter()
        .map(|field| {
            let fn_name = field.ident.as_ref().unwrap();
            let filter_name = format_ident!("where_{}", fn_name);
            let struct_name = format_ident!("{}", fn_name.to_string().to_case(Case::Pascal));
            let field_type = &field.ty;

            let trait_method = quote! {
                fn #filter_name(self, predicate: resent::predicate::QueryPredicate<#field_type>) -> resent::query::EntQuery<'ctx, #ctx_type, #name>;
            };

            let impl_method = quote! {
                fn #filter_name(self, predicate: resent::predicate::QueryPredicate<#field_type>) -> resent::query::EntQuery<'ctx, #ctx_type, #name> {
                    self.filter(
                        <#mod_name::fields::#struct_name as resent::field::EntField<'ctx, #ctx_type, #name>>::predicate(predicate)
                    )
                }
            };

            (trait_method, impl_method)
        })
        .unzip()
}

/// Generates query methods for edge entities, e.g. `query_bar(ctx) -> EntQuery<..., BarEnt>`
fn gen_edge_query_methods(
    edges: &[EntSchemaEdge],
    ctx_type: &syn::Path,
) -> Vec<proc_macro2::TokenStream> {
    edges
        .iter()
        .map(|edge| match edge {
            EntSchemaEdge::To { entity, on, from_field } => {
                let method_name = query_fn_name(entity);
                let where_fn = format_ident!("where_{}", on);
                let from_field = format_ident!("{}", from_field);

                quote! {
                    pub fn #method_name<'ctx>(&self, ctx: &'ctx resent::query::QueryContext<#ctx_type>) -> resent::query::EntQuery<'ctx, #ctx_type, #entity> {
                        use resent::{Ent, query::EntQuery};
                        #entity::query(ctx).#where_fn(resent::predicate::QueryPredicate::Equals(self.#from_field))
                    }
                }
            }
            EntSchemaEdge::From { entity, on, to_field } => {
                let method_name = query_fn_name(entity);
                let where_fn = format_ident!("where_{}", on);
                let to_field = format_ident!("{}", to_field);

                quote! {
                    pub fn #method_name<'ctx>(&self, ctx: &'ctx resent::query::QueryContext<#ctx_type>) -> resent::query::EntQuery<'ctx, #ctx_type, #entity> {
                        use resent::{Ent, query::EntQuery};
                        #entity::query(ctx).#where_fn(resent::predicate::QueryPredicate::Equals(self.#to_field))
                    }
                }
            }
        })
        .collect()
}

/// Generates a loader method for the primary key, e.g. `load(ctx, id) -> Ent`
fn gen_primary_key_loader_method(
    primary_key: &Option<EntPrimaryKey>,
    fields: &[EntStructField],
    ctx_type: &syn::Path,
) -> proc_macro2::TokenStream {
    if let Some(pk) = primary_key {
        match pk {
            EntPrimaryKey::Single(field) => {
                let field_name = format_ident!("{}", field.segments.last().unwrap().ident.to_string());
                let filter_name = format_ident!("where_{}", field_name);
                let field_type = fields.iter().find(|f| f.ident.as_ref().unwrap() == &field_name).unwrap().ty.clone();
                return quote! {
                    pub async fn load<'ctx>(ctx: &'ctx resent::query::QueryContext<#ctx_type>, #field_name: #field_type) -> Result<Self, resent::query::EntLoadError> {
                        use resent::{Ent, query::EntQuery};
                        Self::query(ctx).#filter_name(resent::predicate::QueryPredicate::Equals(#field_name)).load_only().await
                    }
                };
            }
            _ => {},
        };
    }
    
    quote! {}
}

/// Generates query methods for edge entities that return subqueries, e.g. `query_bar() -> EntQuery<..., BarEnt>` where the filter is an IN subquery on the edge field.
fn gen_edge_ent_query_methods(
    edges: &[EntSchemaEdge],
    ctx_type: &syn::Path,
) -> (Vec<proc_macro2::TokenStream>, Vec<proc_macro2::TokenStream>) {
    edges
        .iter()
        .map(|edge| match edge {
            EntSchemaEdge::To { entity, on, from_field } => {
                let method_name = query_fn_name(entity);
                let where_fn = format_ident!("where_{}", on);
                let from_field = format_ident!("{}", from_field);
                
                let trait_method = quote! {
                    fn #method_name(self) -> resent::query::EntQuery<'ctx, #ctx_type, #entity>;
                };
                
                let impl_method = quote! {
                    fn #method_name(self) -> resent::query::EntQuery<'ctx, #ctx_type, #entity> {
                        let (ctx, mut subquery): (&'ctx resent::query::QueryContext<#ctx_type>, sea_query::SelectStatement) = self.into();
                        subquery.clear_selects().column(stringify!(#from_field));
                        use resent::{Ent, query::EntQuery};
                        #entity::query(ctx).#where_fn(resent::predicate::QueryPredicate::InSubquery(subquery))
                    }
                };

                (trait_method, impl_method)
            }
            EntSchemaEdge::From { entity, on, to_field } => {
                let method_name = query_fn_name(entity);
                let where_fn = format_ident!("where_{}", on);
                let to_field = format_ident!("{}", to_field);

                let trait_method = quote! {
                    fn #method_name(self) -> resent::query::EntQuery<'ctx, #ctx_type, #entity>;
                };

                let impl_method = quote! {
                    fn #method_name(self) -> resent::query::EntQuery<'ctx, #ctx_type, #entity> {
                        let (ctx, mut subquery): (&'ctx resent::query::QueryContext<#ctx_type>, sea_query::SelectStatement) = self.into();
                        subquery.clear_selects().column(stringify!(#to_field));
                        use resent::{Ent, query::EntQuery};
                        #entity::query(ctx).#where_fn(resent::predicate::QueryPredicate::InSubquery(subquery))
                    }
                };
                
                (trait_method, impl_method)
            }
        })
        .unzip()
}

/// Converts an entity type path (e.g. `EntBar`) into a query method name (e.g. `query_bar`).
fn query_fn_name(path: &syn::Path) -> proc_macro2::Ident {
    format_ident!(
        "query_{}",
            path
            .segments
            .last()
            .unwrap()
            .ident
            .to_string()
            .to_case(Case::Snake)
            .trim_start_matches("ent_")
    )
}

