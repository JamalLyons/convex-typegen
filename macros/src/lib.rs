#![warn(missing_docs)]
//! Implementation of the derive exposed by `convex-typegen-runtime`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as Tokens;
use quote::{ToTokens, format_ident, quote, quote_spanned};
use syn::ext::IdentExt as _;
use syn::spanned::Spanned as _;
use syn::{
    Attribute, Data, DataEnum, DataStruct, DeriveInput, Error, Expr, ExprLit, ExprUnary, Fields, Ident, Lit, LitInt, LitStr,
    Result, Type, UnOp, parse_macro_input,
};

/// Derive Serde implementations for a concrete Convex model, such as a named-field
/// struct or an enum that models a Convex union.
///
/// Import this derive from `convex_typegen_runtime::ConvexValue`. Use the runtime's
/// `value_serde::to_value` and `value_serde::from_value` to encode and decode the model.
/// The [derive guide](https://jamallyons.github.io/convex-typegen/docs/macros.html)
/// shows how to customize wire names, optional fields, and literal values.
#[proc_macro_derive(ConvexValue, attributes(convex))]
pub fn derive_convex_value(input: TokenStream) -> TokenStream
{
    let input = parse_macro_input!(input as DeriveInput);
    expand(&input).unwrap_or_else(Error::into_compile_error).into()
}

fn expand(input: &DeriveInput) -> Result<Tokens>
{
    if !input.generics.params.is_empty() || input.generics.where_clause.is_some() {
        return Err(Error::new_spanned(
            &input.generics,
            "ConvexValue requires a concrete model type",
        ));
    }

    let name = &input.ident;
    let (definitions, encode, decode) = match &input.data {
        Data::Struct(data) => structure(input, data)?,
        Data::Enum(data) => union(name, data, &input.attrs)?,
        Data::Union(data) => return Err(Error::new(data.union_token.span, "ConvexValue does not support Rust unions")),
    };

    Ok(quote! {
        const _: () = {
            #definitions
            #[automatically_derived]
            impl ::convex_typegen_runtime::serde::Serialize for #name {
                fn serialize<__ConvexSerializer: ::convex_typegen_runtime::serde::Serializer>(
                    &self, __convex_serializer: __ConvexSerializer,
                ) -> ::core::result::Result<__ConvexSerializer::Ok, __ConvexSerializer::Error> {
                    #encode
                }
            }

            #[automatically_derived]
            impl<'de> ::convex_typegen_runtime::serde::Deserialize<'de> for #name {
                fn deserialize<__ConvexDeserializer: ::convex_typegen_runtime::serde::Deserializer<'de>>(
                    __convex_deserializer: __ConvexDeserializer,
                ) -> ::core::result::Result<Self, __ConvexDeserializer::Error> {
                    #decode
                }
            }
        };
    })
}

fn structure(input: &DeriveInput, data: &DataStruct) -> Result<(Tokens, Tokens, Tokens)>
{
    let mut literal = None;
    convex_attributes(&input.attrs, |meta| {
        let value = if meta.path.is_ident("literal") {
            literal_expr(meta.value()?.parse()?)?
        } else if meta.path.is_ident("float_bits") {
            let bits: LitInt = meta.value()?.parse()?;
            let bits = bits.base10_parse::<u64>()?;
            float_literal(bits)
        } else {
            return Err(meta.error("expected literal or float_bits on a ConvexValue struct"));
        };
        set_once(&mut literal, value, &meta.path)
    })?;
    let name = &input.ident;
    if let Some((ty, value, matches)) = literal {
        if !matches!(data.fields, Fields::Unit) {
            return Err(Error::new_spanned(&data.fields, "a Convex literal requires a unit struct"));
        }
        let expected = format!("expected literal for {name}");
        return Ok((
            quote! {},
            quote! { ::convex_typegen_runtime::serde::Serialize::serialize(&(#value), __convex_serializer) },
            quote! {
                let __convex_value = <#ty as ::convex_typegen_runtime::serde::Deserialize>::deserialize(__convex_deserializer)?;
                if #matches {
                    ::core::result::Result::Ok(Self)
                } else {
                    ::core::result::Result::Err(::convex_typegen_runtime::serde::de::Error::custom(#expected))
                }
            },
        ));
    }

    let Fields::Named(fields) = &data.fields else {
        return Err(Error::new(
            name.span(),
            "ConvexValue requires named fields or a unit struct with a literal",
        ));
    };
    let model_name = name.unraw().to_string();
    let remote_path = name.to_string();
    // The helper lives in the same scope as the model's field types. Avoid shadowing any
    // type name referenced in the input, including when the input is itself named this way.
    let source = input.to_token_stream().to_string();
    let mut remote = format_ident!("__ConvexValueObject");
    while source.contains(&remote.to_string()) {
        remote = format_ident!("{remote}_");
    }
    let mut definitions = Vec::new();
    let mut wire_names = std::collections::BTreeSet::new();
    for field in &fields.named {
        let ident = field.ident.as_ref().unwrap();
        let ty = &field.ty;
        let mut rename = None;
        let mut optional = None;
        convex_attributes(&field.attrs, |meta| {
            if meta.path.is_ident("rename") {
                set_once(&mut rename, meta.value()?.parse::<LitStr>()?, &meta.path)
            } else if meta.path.is_ident("optional") {
                set_once(&mut optional, (), &meta.path)
            } else {
                Err(meta.error("expected rename or optional on a ConvexValue field"))
            }
        })?;
        let wire = rename.unwrap_or_else(|| LitStr::new(&ident.unraw().to_string(), ident.span()));
        if !wire_names.insert(wire.value()) {
            return Err(Error::new(wire.span(), "duplicate Convex wire field"));
        }

        // A deserialize hook makes a field required even when its Rust type is Option<T>.
        let presence = if optional.is_some() {
            quote! {
                #[serde(default, skip_serializing_if = "::core::option::Option::is_none",
                        deserialize_with = "::convex_typegen_runtime::model::deserialize_present")]
            }
        } else {
            quote! { #[serde(deserialize_with = "::convex_typegen_runtime::serde::Deserialize::deserialize")] }
        };
        definitions.push(quote_spanned! {field.span()=>
            #[serde(rename = #wire)]
            #presence
            #ident: #ty
        });
    }
    Ok((
        quote! {
            #[derive(::convex_typegen_runtime::serde::Serialize, ::convex_typegen_runtime::serde::Deserialize)]
            #[serde(crate = "::convex_typegen_runtime::serde", remote = #remote_path,
                    rename = #model_name, deny_unknown_fields)]
            struct #remote { #(#definitions,)* }
        },
        quote! { #remote::serialize(self, __convex_serializer) },
        quote! { #remote::deserialize(__convex_deserializer) },
    ))
}

fn literal_expr(expr: Expr) -> Result<(Tokens, Tokens, Tokens)>
{
    let (negative, lit) = match &expr {
        Expr::Lit(ExprLit { lit, .. }) => (false, lit),
        Expr::Unary(ExprUnary {
            op: UnOp::Neg(_), expr, ..
        }) => {
            let Expr::Lit(ExprLit { lit, .. }) = expr.as_ref() else {
                return Err(Error::new_spanned(expr, "expected a numeric literal"));
            };
            (true, lit)
        }
        _ => return Err(Error::new_spanned(expr, "expected a Convex literal")),
    };
    match lit {
        Lit::Str(_) if !negative => Ok((
            quote! { ::std::string::String },
            quote! { #lit },
            quote! { __convex_value == #lit },
        )),
        Lit::Bool(_) if !negative => Ok((quote! { bool }, quote! { #lit }, quote! { __convex_value == #lit })),
        Lit::Int(int) if matches!(int.suffix(), "" | "i64") => {
            let digits = int.base10_digits();
            let signed = if negative { format!("-{digits}") } else { digits.into() };
            let number: i64 = signed
                .parse()
                .map_err(|_| Error::new_spanned(&expr, "literal is outside the i64 range"))?;
            Ok((quote! { i64 }, quote! { #number }, quote! { __convex_value == #number }))
        }
        Lit::Float(float) if matches!(float.suffix(), "" | "f64") => {
            let number: f64 = float.base10_parse()?;
            Ok(float_literal(if negative { -number } else { number }.to_bits()))
        }
        _ => Err(Error::new_spanned(expr, "expected a string, bool, i64 or f64 literal")),
    }
}

fn float_literal(bits: u64) -> (Tokens, Tokens, Tokens)
{
    (
        quote! { f64 },
        quote! { f64::from_bits(#bits) },
        quote! { __convex_value.to_bits() == #bits },
    )
}

fn union(name: &Ident, data: &DataEnum, attrs: &[Attribute]) -> Result<(Tokens, Tokens, Tokens)>
{
    let mut discriminator = None;
    convex_attributes(attrs, |meta| {
        if meta.path.is_ident("discriminator") {
            set_once(&mut discriminator, meta.value()?.parse::<LitStr>()?, &meta.path)
        } else {
            Err(meta.error("expected discriminator on a ConvexValue enum"))
        }
    })?;
    if data.variants.is_empty() {
        return Err(Error::new(name.span(), "a ConvexValue enum requires at least one branch"));
    }
    if data.variants.iter().all(|variant| matches!(variant.fields, Fields::Unit)) {
        if let Some(discriminator) = discriminator {
            return Err(Error::new(
                discriminator.span(),
                "a string literal enum cannot have a discriminator",
            ));
        }
        return string_enum(name, data);
    }
    let mut encode = Vec::new();
    let mut decode = Vec::new();
    for variant in &data.variants {
        let ident = &variant.ident;
        let Fields::Unnamed(fields) = &variant.fields else {
            return Err(Error::new(ident.span(), "a ConvexValue branch requires one unnamed field"));
        };
        if fields.unnamed.len() != 1 || variant.discriminant.is_some() {
            return Err(Error::new_spanned(variant, "a ConvexValue branch requires one unnamed field"));
        }
        let field = &fields.unnamed[0];
        convex_attributes(&field.attrs, |meta| Err(meta.error("put the tag on the enum variant")))?;
        let ty = &field.ty;
        let mut tag = None;
        convex_attributes(&variant.attrs, |meta| {
            if meta.path.is_ident("tag") && discriminator.is_some() {
                set_once(&mut tag, meta.value()?.parse::<Type>()?, &meta.path)
            } else {
                Err(meta.error("tag requires a discriminator on the enum"))
            }
        })?;
        encode.push(quote! {
            Self::#ident(__convex_value) => ::convex_typegen_runtime::serde::Serialize::serialize(__convex_value, __convex_serializer)
        });
        let codec = quote! { ::convex_typegen_runtime::value_serde::from_value };
        if let Some(key) = &discriminator {
            let tag = tag.ok_or_else(|| Error::new(ident.span(), "missing tag type for discriminated branch"))?;
            decode.push(quote! {
                if __convex_raw.field(#key).is_some_and(|__convex_tag| #codec::<#tag>(__convex_tag.clone()).is_ok()) {
                    return #codec::<#ty>(__convex_raw.0).map(Self::#ident)
                        .map_err(::convex_typegen_runtime::serde::de::Error::custom);
                }
            });
        } else {
            decode.push(quote! {
                if let ::core::result::Result::Ok(__convex_value) = #codec::<#ty>(__convex_raw.0.clone()) {
                    return ::core::result::Result::Ok(Self::#ident(__convex_value));
                }
            });
        }
    }
    let expected = format!("no matching branch of {name}");
    Ok((
        quote! {},
        quote! { match self { #(#encode,)* } },
        quote! {
            let __convex_raw = <::convex_typegen_runtime::model::RawValue as
                ::convex_typegen_runtime::serde::Deserialize>::deserialize(__convex_deserializer)?;
            #(#decode)*
            ::core::result::Result::Err(::convex_typegen_runtime::serde::de::Error::custom(#expected))
        },
    ))
}

fn string_enum(name: &Ident, data: &DataEnum) -> Result<(Tokens, Tokens, Tokens)>
{
    let mut seen = std::collections::BTreeSet::new();
    let mut encode = Vec::new();
    let mut decode = Vec::new();
    for variant in &data.variants {
        let ident = &variant.ident;
        let mut literal = None;
        convex_attributes(&variant.attrs, |meta| {
            if meta.path.is_ident("literal") {
                set_once(&mut literal, meta.value()?.parse::<LitStr>()?, &meta.path)
            } else {
                Err(meta.error("expected a string literal on a unit variant"))
            }
        })?;
        let literal = literal.ok_or_else(|| Error::new(ident.span(), "missing string literal on unit variant"))?;
        if !seen.insert(literal.value()) {
            return Err(Error::new(literal.span(), "duplicate string literal"));
        }
        encode.push(quote! { Self::#ident => #literal });
        decode.push(quote! { #literal => ::core::result::Result::Ok(Self::Value::#ident) });
    }
    let mut visitor = format_ident!("__ConvexStringLiteralVisitor");
    if name == &visitor {
        visitor = format_ident!("__ConvexStringLiteralVisitor_");
    }
    let expected = format!("a string literal of {name}");
    let unknown = format!("no matching string literal of {name}");
    Ok((
        quote! {},
        quote! { __convex_serializer.serialize_str(match self { #(#encode,)* }) },
        quote! {
            struct #visitor;

            impl<'de> ::convex_typegen_runtime::serde::de::Visitor<'de> for #visitor {
                type Value = #name;

                fn expecting(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    formatter.write_str(#expected)
                }

                fn visit_str<E: ::convex_typegen_runtime::serde::de::Error>(
                    self, value: &str,
                ) -> ::core::result::Result<Self::Value, E> {
                    match value {
                        #(#decode,)*
                        _ => ::core::result::Result::Err(E::custom(#unknown)),
                    }
                }
            }

            __convex_deserializer.deserialize_str(#visitor)
        },
    ))
}

fn convex_attributes(attrs: &[Attribute], mut parse: impl FnMut(syn::meta::ParseNestedMeta<'_>) -> Result<()>)
-> Result<()>
{
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("convex")) {
        attr.parse_nested_meta(&mut parse)?;
    }
    Ok(())
}

fn set_once<T>(slot: &mut Option<T>, value: T, at: &impl syn::spanned::Spanned) -> Result<()>
{
    if slot.replace(value).is_some() {
        return Err(Error::new(at.span(), "duplicate ConvexValue attribute"));
    }
    Ok(())
}
