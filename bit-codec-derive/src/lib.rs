use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Fields, Ident, LitInt, Type};

/// Derive `BitEncode` for structs and enums.
///
/// # Supported forms
///
/// - **Unit structs**: `struct Marker`; zero bits on the wire
/// - **Named structs**: `struct Foo { x: u8, y: u8 }`
/// - **Unnamed structs**: `struct Wrapper(u16)`, `struct Pair(u16, u16)`
/// - **Enums**: unit, tuple, and struct variants
///
/// # Struct attributes
///
/// - `#[bits(N)]` on a field: write/read this field as N bits via
///   `write_bits`/`read_bits` instead of calling the trait method.
///
/// # Enum attributes
///
/// - `#[bits(disc = N)]` on the enum: encode the variant index as N bits.
///   Defaults to 8 if omitted.
/// - Supports unit variants, tuple variants (one or more fields), and
///   struct variants.
///
/// # Example
///
/// ```ignore
/// #[derive(BitEncode, BitDecode)]
/// struct Header {
///     #[bits(5)]
///     version: u8,
///     #[bits(3)]
///     flags: u8,
///     name: String,
/// }
///
/// #[derive(BitEncode, BitDecode)]
/// #[bits(disc = 4)]
/// enum Packet {
///     Ping,
///     Data { payload: Vec<u8> },
///     Error(String),
/// }
/// ```
#[proc_macro_derive(BitEncode, attributes(bits))]
pub fn derive_bit_encode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_encode(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Derive `BitDecode`. See [`BitEncode`] for attribute docs.
#[proc_macro_derive(BitDecode, attributes(bits))]
pub fn derive_bit_decode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_decode(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Parse `#[bits(N)]` from a field's attributes. Returns `None` if absent.
fn field_bit_count(attrs: &[Attribute]) -> syn::Result<Option<u32>> {
    for attr in attrs {
        if !attr.path().is_ident("bits") {
            continue;
        }
        let lit: LitInt = attr.parse_args()?;
        let n: u32 = lit.base10_parse()?;
        if n == 0 || n > 128 {
            return Err(syn::Error::new_spanned(lit, "bit count must be 1..=128"));
        }
        return Ok(Some(n));
    }
    Ok(None)
}

/// Parse `#[bits(disc = N)]` from the enum-level attributes. Defaults to 8.
fn disc_bit_count(attrs: &[Attribute]) -> syn::Result<u32> {
    for attr in attrs {
        if !attr.path().is_ident("bits") {
            continue;
        }
        let nv: syn::MetaNameValue = attr.parse_args()?;
        if !nv.path.is_ident("disc") {
            return Err(syn::Error::new_spanned(nv.path, "expected `disc = N`"));
        }
        if let syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(ref lit),
            ..
        }) = nv.value
        {
            let n: u32 = lit.base10_parse()?;
            if n == 0 || n > 32 {
                return Err(syn::Error::new_spanned(lit, "disc bits must be 1..=32"));
            }
            return Ok(n);
        }
        return Err(syn::Error::new_spanned(
            nv.value,
            "expected integer literal",
        ));
    }
    Ok(8)
}

fn encode_field(
    accessor: &TokenStream2,
    ty: &Type,
    attrs: &[Attribute],
) -> syn::Result<TokenStream2> {
    if let Some(n) = field_bit_count(attrs)? {
        Ok(quote! { w.write_bits(#accessor, #n)?; })
    } else {
        Ok(quote! { <#ty as bit_codec::BitEncode>::encode(&#accessor, w)?; })
    }
}

fn decode_field(ty: &Type, attrs: &[Attribute]) -> syn::Result<TokenStream2> {
    if let Some(n) = field_bit_count(attrs)? {
        Ok(quote! { r.read_bits(#n)? })
    } else {
        Ok(quote! { <#ty as bit_codec::BitDecode>::decode(r)? })
    }
}

fn expand_encode(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let body = match &input.data {
        Data::Struct(ds) => encode_struct_body(&ds.fields)?,
        Data::Enum(de) => encode_enum_body(name, &de.variants, &input.attrs)?,
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(name, "unions are not supported"));
        }
    };

    Ok(quote! {
        impl #impl_generics bit_codec::BitEncode for #name #ty_generics #where_clause {
            fn encode<W__: std::io::Write>(&self, w: &mut bit_codec::BitWriter<W__>) -> std::io::Result<()> {
                #body
                Ok(())
            }
        }
    })
}

fn encode_struct_body(fields: &Fields) -> syn::Result<TokenStream2> {
    match fields {
        Fields::Named(named) => {
            let stmts: Vec<_> = named
                .named
                .iter()
                .map(|f| {
                    let ident = f.ident.as_ref().unwrap();
                    let accessor = quote! { self.#ident };
                    encode_field(&accessor, &f.ty, &f.attrs)
                })
                .collect::<syn::Result<_>>()?;
            Ok(quote! { #(#stmts)* })
        }
        Fields::Unnamed(unnamed) => {
            let stmts: Vec<_> = unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let idx = syn::Index::from(i);
                    let accessor = quote! { self.#idx };
                    encode_field(&accessor, &f.ty, &f.attrs)
                })
                .collect::<syn::Result<_>>()?;
            Ok(quote! { #(#stmts)* })
        }
        Fields::Unit => Ok(TokenStream2::new()),
    }
}

fn encode_enum_body(
    _name: &Ident,
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
    attrs: &[Attribute],
) -> syn::Result<TokenStream2> {
    let disc_bits = disc_bit_count(attrs)?;
    let arms: Vec<_> = variants
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let vname = &v.ident;
            let disc = i as u32;
            match &v.fields {
                Fields::Unit => Ok(quote! {
                    Self::#vname => {
                        w.write_bits(#disc as u32, #disc_bits)?;
                    }
                }),
                Fields::Named(named) => {
                    let bindings: Vec<_> = named
                        .named
                        .iter()
                        .map(|f| {
                            let id = f.ident.as_ref().unwrap();
                            quote! { #id }
                        })
                        .collect();
                    let stmts: Vec<_> = named
                        .named
                        .iter()
                        .map(|f| {
                            let id = f.ident.as_ref().unwrap();
                            let accessor = quote! { *#id };
                            encode_field(&accessor, &f.ty, &f.attrs)
                        })
                        .collect::<syn::Result<_>>()?;
                    Ok(quote! {
                        Self::#vname { #(#bindings),* } => {
                            w.write_bits(#disc as u32, #disc_bits)?;
                            #(#stmts)*
                        }
                    })
                }
                Fields::Unnamed(unnamed) => {
                    let bindings: Vec<_> = (0..unnamed.unnamed.len())
                        .map(|i| Ident::new(&format!("_{i}"), proc_macro2::Span::call_site()))
                        .collect();
                    let stmts: Vec<_> = unnamed
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(i, f)| {
                            let b = &bindings[i];
                            let accessor = quote! { *#b };
                            encode_field(&accessor, &f.ty, &f.attrs)
                        })
                        .collect::<syn::Result<_>>()?;
                    Ok(quote! {
                        Self::#vname(#(#bindings),*) => {
                            w.write_bits(#disc as u32, #disc_bits)?;
                            #(#stmts)*
                        }
                    })
                }
            }
        })
        .collect::<syn::Result<_>>()?;

    Ok(quote! {
        match self {
            #(#arms)*
        }
    })
}

fn expand_decode(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let body = match &input.data {
        Data::Struct(ds) => decode_struct_body(name, &ds.fields)?,
        Data::Enum(de) => decode_enum_body(name, &de.variants, &input.attrs)?,
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(name, "unions are not supported"));
        }
    };

    Ok(quote! {
        impl #impl_generics bit_codec::BitDecode for #name #ty_generics #where_clause {
            fn decode<R__: std::io::Read>(r: &mut bit_codec::BitReader<R__>) -> std::io::Result<Self> {
                #body
            }
        }
    })
}

fn decode_struct_body(name: &Ident, fields: &Fields) -> syn::Result<TokenStream2> {
    match fields {
        Fields::Named(named) => {
            let field_inits: Vec<_> = named
                .named
                .iter()
                .map(|f| {
                    let ident = f.ident.as_ref().unwrap();
                    let expr = decode_field(&f.ty, &f.attrs)?;
                    Ok(quote! { #ident: #expr })
                })
                .collect::<syn::Result<_>>()?;
            Ok(quote! { Ok(#name { #(#field_inits,)* }) })
        }
        Fields::Unnamed(unnamed) => {
            let field_exprs: Vec<_> = unnamed
                .unnamed
                .iter()
                .map(|f| decode_field(&f.ty, &f.attrs))
                .collect::<syn::Result<_>>()?;
            Ok(quote! { Ok(#name( #(#field_exprs,)* )) })
        }
        Fields::Unit => Ok(quote! { Ok(#name) }),
    }
}

fn decode_enum_body(
    _name: &Ident,
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
    attrs: &[Attribute],
) -> syn::Result<TokenStream2> {
    let disc_bits = disc_bit_count(attrs)?;
    let arms: Vec<_> = variants
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let vname = &v.ident;
            let disc = i as u32;
            let construct = match &v.fields {
                Fields::Unit => quote! { Self::#vname },
                Fields::Named(named) => {
                    let inits: Vec<_> = named
                        .named
                        .iter()
                        .map(|f| {
                            let id = f.ident.as_ref().unwrap();
                            let expr = decode_field(&f.ty, &f.attrs)?;
                            Ok(quote! { #id: #expr })
                        })
                        .collect::<syn::Result<_>>()?;
                    quote! { Self::#vname { #(#inits,)* } }
                }
                Fields::Unnamed(unnamed) => {
                    let exprs: Vec<_> = unnamed
                        .unnamed
                        .iter()
                        .map(|f| decode_field(&f.ty, &f.attrs))
                        .collect::<syn::Result<_>>()?;
                    quote! { Self::#vname(#(#exprs,)*) }
                }
            };
            Ok(quote! { #disc => Ok(#construct), })
        })
        .collect::<syn::Result<_>>()?;

    Ok(quote! {
        let disc: u32 = r.read_bits(#disc_bits)?;
        match disc {
            #(#arms)*
            other => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown variant discriminant: {}", other),
            )),
        }
    })
}
