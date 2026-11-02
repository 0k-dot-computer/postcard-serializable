use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, parse_quote, DeriveInput};

/// Derive macro that implements `PostcardSerializable` with compile-time validation.
///
/// This macro requires the type to already derive `Serialize`, `Deserialize`, and `Default`.
/// It adds the `PostcardSerializable` marker trait and validates at compile time that
/// the type and its wire-visible fields use types that have opted into postcard
/// serialization.
///
/// # Example
/// ```
/// use serde::{Serialize, Deserialize};
/// use postcard_serializable::PostcardSerializable;
/// use postcard_serializable_types::PostcardSerializable as PostcardSerializableTrait;
///
/// #[derive(Debug, Clone, Default, Serialize, Deserialize, PostcardSerializable)]
/// struct MyEvent {
///     id: u64,
///     data: Vec<u8>,
/// }
///
/// // The type now implements PostcardSerializableTrait
/// fn use_it<T: PostcardSerializableTrait>(_: &T) {}
/// ```
///
/// # Compile-time validation
///
/// The macro generates a const block that requires concrete, wire-visible fields
/// to implement `PostcardSerializable`. This catches issues like:
/// - `#[serde(skip_serializing_if = "...")]` which is incompatible with postcard
/// - Types that use `deserialize_any()` like `serde_json::Value`
///
/// # Errors
///
/// Compilation will fail if:
/// - The type contains known incompatible types (e.g., `serde_json::Value`)
/// - A concrete wire-visible field does not implement `PostcardSerializable`
#[proc_macro_derive(PostcardSerializable)]
pub fn derive_postcard_serializable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Static analysis to catch known incompatible types
    let validation_result = validate_postcard_at_compile_time(&input);
    if let Err(error) = validation_result {
        return syn::Error::new_spanned(name, error)
            .to_compile_error()
            .into();
    }

    let field_types = collect_field_types(&input);
    let generic_param_names = generic_param_names(&input.generics);
    let mut impl_generics_with_bounds = input.generics.clone();
    {
        let where_clause = impl_generics_with_bounds.make_where_clause();
        for field_type in &field_types {
            if type_mentions_ident(field_type, name) {
                continue;
            }
            if !type_mentions_any_ident(field_type, &generic_param_names) {
                continue;
            }
            where_clause
                .predicates
                .push(parse_quote!(#field_type: postcard_serializable_types::PostcardSerializable));
        }
    }
    let (impl_generics, ty_generics, where_clause) = impl_generics_with_bounds.split_for_impl();
    let concrete_field_assertions = field_types
        .iter()
        .filter(|field_type| !type_mentions_any_ident(field_type, &generic_param_names))
        .map(|field_type| {
            quote! {
                require_postcard_serializable::<#field_type>();
            }
        });
    let type_assertion = if input.generics.params.is_empty() {
        quote! {
            require_postcard_serializable::<#name #ty_generics>();
        }
    } else {
        quote! {}
    };

    quote! {
        // Implement the marker trait for this type
        impl #impl_generics postcard_serializable_types::PostcardSerializable for #name #ty_generics #where_clause {}

        // Compile-time validation by generating const code that must type-check.
        const _: () = {
            const fn require_postcard_serializable<T: postcard_serializable_types::PostcardSerializable>() {}

            #( #concrete_field_assertions )*
            #type_assertion
        };
    }
    .into()
}

/// Validate postcard compatibility via static analysis
fn validate_postcard_at_compile_time(input: &DeriveInput) -> Result<(), String> {
    if let Err(e) = check_serde_attrs(&input.attrs) {
        return Err(format!("Type '{}': {}", input.ident, e));
    }

    // Check the struct fields for known incompatible types and attributes
    match &input.data {
        syn::Data::Struct(data_struct) => match &data_struct.fields {
            syn::Fields::Named(fields) => {
                for field in &fields.named {
                    let field_name = field.ident.as_ref().unwrap();
                    if let Err(e) = check_field_type(&field.ty) {
                        return Err(format!("Field '{}': {}", field_name, e));
                    }
                    if let Err(e) = check_serde_attrs(&field.attrs) {
                        return Err(format!("Field '{}': {}", field_name, e));
                    }
                }
            }
            syn::Fields::Unnamed(fields) => {
                for (i, field) in fields.unnamed.iter().enumerate() {
                    if let Err(e) = check_field_type(&field.ty) {
                        return Err(format!("Field {}: {}", i, e));
                    }
                    if let Err(e) = check_serde_attrs(&field.attrs) {
                        return Err(format!("Field {}: {}", i, e));
                    }
                }
            }
            syn::Fields::Unit => {}
        },
        syn::Data::Enum(data_enum) => {
            for variant in &data_enum.variants {
                if let Err(e) = check_serde_attrs(&variant.attrs) {
                    return Err(format!("Variant {}::{}: {}", input.ident, variant.ident, e));
                }
                match &variant.fields {
                    syn::Fields::Named(fields) => {
                        for field in &fields.named {
                            let field_name = field.ident.as_ref().unwrap();
                            if let Err(e) = check_field_type(&field.ty) {
                                return Err(format!(
                                    "Variant {}::{}: {}",
                                    input.ident, field_name, e
                                ));
                            }
                            if let Err(e) = check_serde_attrs(&field.attrs) {
                                return Err(format!(
                                    "Variant {}::{}: {}",
                                    input.ident, field_name, e
                                ));
                            }
                        }
                    }
                    syn::Fields::Unnamed(fields) => {
                        for (i, field) in fields.unnamed.iter().enumerate() {
                            if let Err(e) = check_field_type(&field.ty) {
                                return Err(format!("Variant {}::{}: {}", input.ident, i, e));
                            }
                            if let Err(e) = check_serde_attrs(&field.attrs) {
                                return Err(format!("Variant {}::{}: {}", input.ident, i, e));
                            }
                        }
                    }
                    syn::Fields::Unit => {}
                }
            }
        }
        syn::Data::Union(_) => {
            return Err("Unions are not supported for postcard serialization".to_string());
        }
    }

    Ok(())
}

fn collect_field_types(input: &DeriveInput) -> Vec<syn::Type> {
    let mut field_types = Vec::new();
    match &input.data {
        syn::Data::Struct(data_struct) => collect_fields(&data_struct.fields, &mut field_types),
        syn::Data::Enum(data_enum) => {
            for variant in &data_enum.variants {
                collect_fields(&variant.fields, &mut field_types);
            }
        }
        syn::Data::Union(_) => {}
    }
    field_types
}

fn collect_fields(fields: &syn::Fields, field_types: &mut Vec<syn::Type>) {
    match fields {
        syn::Fields::Named(fields) => {
            for field in &fields.named {
                if serde_attrs_contain_ident(&field.attrs, "skip")
                    || serde_attrs_contain_ident(&field.attrs, "with")
                {
                    continue;
                }
                field_types.push(field.ty.clone());
            }
        }
        syn::Fields::Unnamed(fields) => {
            for field in &fields.unnamed {
                if serde_attrs_contain_ident(&field.attrs, "skip")
                    || serde_attrs_contain_ident(&field.attrs, "with")
                {
                    continue;
                }
                field_types.push(field.ty.clone());
            }
        }
        syn::Fields::Unit => {}
    }
}

fn generic_param_names(generics: &syn::Generics) -> Vec<String> {
    generics
        .params
        .iter()
        .map(|param| match param {
            syn::GenericParam::Type(param) => param.ident.to_string(),
            syn::GenericParam::Const(param) => param.ident.to_string(),
            syn::GenericParam::Lifetime(param) => param.lifetime.ident.to_string(),
        })
        .collect()
}

fn type_mentions_any_ident(ty: &syn::Type, idents: &[String]) -> bool {
    idents
        .iter()
        .any(|ident| type_mentions_ident_str(ty, ident))
}

fn type_mentions_ident(ty: &syn::Type, ident: &syn::Ident) -> bool {
    type_mentions_ident_str(ty, &ident.to_string())
}

fn type_mentions_ident_str(ty: &syn::Type, ident: &str) -> bool {
    quote!(#ty)
        .to_string()
        .split(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
        .any(|token| token == ident)
}

/// Check field attributes for postcard-incompatible serde annotations
fn check_serde_attrs(attrs: &[syn::Attribute]) -> Result<(), String> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let mut error = None;
        if attr
            .parse_nested_meta(|meta| {
                if let Some(message) = incompatible_serde_attr_message(&meta.path) {
                    error = Some(message);
                }
                Ok(())
            })
            .is_err()
        {
            let tokens = attr.to_token_stream().to_string();
            for name in [
                "skip_serializing_if",
                "skip_serializing",
                "skip_deserializing",
                "flatten",
            ] {
                if tokens.contains(name) {
                    error = Some(incompatible_serde_attr_name_message(name));
                    break;
                }
            }
        }
        if let Some(message) = error {
            return Err(message);
        }
    }
    Ok(())
}

fn incompatible_serde_attr_message(path: &syn::Path) -> Option<String> {
    for name in [
        "skip_serializing_if",
        "skip_serializing",
        "skip_deserializing",
        "flatten",
    ] {
        if path.is_ident(name) {
            return Some(incompatible_serde_attr_name_message(name));
        }
    }
    None
}

fn incompatible_serde_attr_name_message(name: &str) -> String {
    format!(
        "#[serde({name} ...)] is incompatible with postcard wire types. \
         Postcard is a positional format, so fields and variants must always \
         serialize in a stable order. Use #[serde(default)] alone for optional fields."
    )
}

fn serde_attrs_contain_ident(attrs: &[syn::Attribute], target: &str) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("serde") {
            return false;
        }
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(target) {
                found = true;
            }
            Ok(())
        });
        found
    })
}

fn check_field_type(ty: &syn::Type) -> Result<(), String> {
    let type_string = quote!(#ty).to_string();

    // Check for known incompatible types
    if type_string.contains("serde_json :: Value") || type_string.contains("serde_json::Value") {
        return Err("serde_json::Value is not compatible with postcard serialization. \
            Postcard requires concrete types, but Value uses deserialize_any() which is not supported."
            .to_string());
    }

    if type_string.contains("Box < dyn") || type_string.contains("Box<dyn") {
        return Err(
            "Trait objects (Box<dyn Trait>) are not compatible with postcard serialization."
                .to_string(),
        );
    }

    Ok(())
}
