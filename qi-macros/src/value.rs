use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    parse_quote, punctuated::Punctuated, spanned::Spanned, Attribute, Data, DataStruct,
    DeriveInput, Error, Fields, GenericParam, Generics, Ident, Index, Lifetime, LifetimeParam,
    LitStr, Path, Token, Type, TypeParamBound, WhereClause,
};

pub(crate) fn derive_impl(derive: Derive, input: DeriveInput) -> syn::Result<TokenStream> {
    Ok(Container::new(derive, input)?.derive_impl())
}

pub(crate) struct Container {
    derive: Derive,
    name: Ident,
    crate_path: Path,
    generics: Generics,
    data: ContainerData,
}

impl Container {
    pub(crate) fn new(derive: Derive, input: DeriveInput) -> syn::Result<Self> {
        let attrs = ContainerAttributes::new(&input.attrs)?;
        let name = input.ident;
        let crate_path = attrs.crate_path;
        let generics = input.generics;
        let data = match input.data {
            Data::Struct(data) => {
                if attrs.transparent {
                    ContainerData::new_struct_transparent(data)
                        .map_err(|err| Error::new_spanned(&name, err))
                } else {
                    Ok(ContainerData::new_struct(
                        name.to_string(),
                        data,
                        attrs.rename_all,
                    ))
                }
            }
            Data::Enum(_) | Data::Union(_) => Err(Error::new_spanned(
                &name,
                format!("{} cannot be derived on enums and unions", derive),
            )),
        }?;
        Ok(Self {
            derive,
            name,
            crate_path,
            generics,
            data,
        })
    }

    fn derive_impl(&self) -> TokenStream {
        match self.derive {
            Derive::Reflect => self.impl_reflect(),
            Derive::AsValue => self.impl_as_value(),
            Derive::FromValue => self.impl_from_value(),
            Derive::StdTryFromValue => self.impl_std_try_from_value(),
            Derive::IntoValue => self.impl_into_value(),
        }
    }

    fn impl_reflect(&self) -> TokenStream {
        let qi = &self.crate_path;
        let name = &self.name;

        let mut generics = self.generics.clone();
        let reflect_bound: TypeParamBound = parse_quote!(#qi ::Reflect);
        for param in generics.type_params_mut() {
            param.bounds.push(reflect_bound.clone());
        }

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let reflect_ty = self.reflect_ty();

        quote! {
            impl #impl_generics #qi ::Reflect for #name #ty_generics #where_clause {
                fn ty() -> Option<#qi ::Type> {
                    #reflect_ty
                }
            }
        }
    }

    fn impl_as_value(&self) -> TokenStream {
        let qi = &self.crate_path;
        let name = &self.name;

        let mut generics = self.generics.clone();
        let as_value_bound: TypeParamBound = parse_quote!(#qi ::AsValue);
        for param in generics.type_params_mut() {
            param.bounds.push(as_value_bound.clone());
        }

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let value_type = self.value_type();
        let as_value = self.as_value();

        quote! {
            impl #impl_generics #qi ::AsValue for #name #ty_generics #where_clause {
                fn value_type(&self) -> #qi ::Type {
                    #value_type
                }

                fn as_value(&self) -> #qi:: Value<'_> {
                    #as_value
                }
            }
        }
    }

    fn impl_from_value(&self) -> TokenStream {
        let qi = &self.crate_path;
        let name = &self.name;

        let value_lt = qi_value_lifetime();

        let mut generics = self.generics.clone();

        // Add a bound `'a: 'value` for each lifetime parameter 'a.
        for lt in generics.lifetimes_mut() {
            lt.bounds.push(value_lt.clone());
        }

        // Add a bound `T: FromValue` for each type parameter T.
        for ty in generics.type_params_mut() {
            ty.bounds.push(parse_quote!(#qi ::FromValue<#value_lt>));
        }

        let (impl_generics, ty_generics, where_clause) =
            split_value_lt_generics(value_lt.clone(), &generics);
        let value_ident = parse_quote!(value);
        let from_value = self.qi_from_value(&value_ident);
        quote! {
            impl #impl_generics #qi ::FromValue<#value_lt> for #name #ty_generics #where_clause {
                fn from_value(#value_ident: #qi ::Value<#value_lt>) -> Result<Self, #qi ::FromValueError> {
                    #from_value
                }
            }
        }
    }

    fn impl_std_try_from_value(&self) -> TokenStream {
        let qi = &self.crate_path;
        let name = &self.name;
        let value_lt = qi_value_lifetime();
        let value_ty = quote!(#qi ::Value<#value_lt>);

        let mut generics = self.generics.clone();

        // Add a bound `'a: 'value` for each lifetime parameter 'a.
        for lt in generics.lifetimes_mut() {
            lt.bounds.push(value_lt.clone());
        }

        // Add a bound `T: FromValue` for each type parameter T.
        for ty in generics.type_params_mut() {
            ty.bounds.push(parse_quote!(#qi ::FromValue<#value_lt>));
        }

        let (impl_generics, ty_generics, where_clause) =
            split_value_lt_generics(value_lt.clone(), &generics);

        quote! {
            impl #impl_generics std::convert::TryFrom< #value_ty > for #name #ty_generics #where_clause {
                type Error = #qi ::FromValueError;
                fn try_from(value: #value_ty) -> Result<Self, Self::Error> {
                    #qi ::FromValue::from_value(value)
                }
            }
        }
    }

    fn impl_into_value(&self) -> TokenStream {
        let qi = &self.crate_path;
        let name = &self.name;
        let value_ident = parse_quote!(value);
        let value_lt = qi_value_lifetime();

        let mut generics = self.generics.clone();

        // Add a bound `T: Into<Value>` for each type parameter T.
        for ty in generics.type_params_mut() {
            ty.bounds
                .push(parse_quote!(std::convert::Into<#qi ::Value< #value_lt >>));
        }

        let (impl_generics, ty_generics, where_clause) =
            split_value_lt_generics(value_lt.clone(), &generics);

        let mut where_clause = where_clause.cloned().unwrap_or_else(|| WhereClause {
            where_token: <Token![where]>::default(),
            predicates: Punctuated::new(),
        });

        // Add a lifetime bound `'value: 'a` for each lifetime parameter 'a.
        let lifetimes = generics
            .lifetimes()
            .map(|LifetimeParam { lifetime, .. }| lifetime);
        let value_lt_where_predicate = parse_quote!(#value_lt: #( #lifetimes )+*);
        where_clause.predicates.push(value_lt_where_predicate);

        let this_ty = quote!(#name #ty_generics);

        let value_from = self.std_into_value(&value_ident);
        quote! {
            impl #impl_generics std::convert::From<#this_ty> for #qi ::Value<#value_lt> #where_clause {
                fn from(#value_ident: #this_ty) -> Self {
                    #value_from
                }
            }
        }
    }

    fn reflect_ty(&self) -> TokenStream {
        let qi = &self.crate_path;
        match &self.data {
            ContainerData::Empty => {
                quote!(#qi ::Type::Unit)
            }
            ContainerData::Field { field } => reflect_ty(qi, &field.ty),
            ContainerData::Struct { name, fields } => {
                let fields = fields
                    .iter()
                    .map(|field| struct_field(qi, field, reflect_ty(qi, &field.ty)));
                quote! {
                    Some(#qi ::Type::Tuple(
                        #qi ::ty::Tuple::Struct {
                            name: #name.to_owned(),
                            fields: vec![ #( #fields ),* ],
                        }
                    ))
                }
            }
            ContainerData::TupleStruct { name, fields } => {
                let fields = fields.iter().map(|element| reflect_ty(qi, element));
                quote! {
                    Some(#qi ::Type::Tuple(
                        #qi ::ty::Tuple::TupleStruct {
                            name: #name.to_owned(),
                            elements: vec![ #( #fields ),* ],
                        }
                    ))
                }
            }
        }
    }

    fn value_type(&self) -> TokenStream {
        let qi = &self.crate_path;
        match &self.data {
            ContainerData::Empty => quote!(#qi ::Type::Unit),
            ContainerData::Field { field } => {
                let field_access = field_access(quote!(self), field.ident.as_ref(), 0);
                quote!(#qi ::AsValue::value_type(& #field_access))
            }
            ContainerData::Struct { name, fields } => {
                let fields = fields.iter().enumerate().map(|(idx, field)| {
                    let value_type =
                        value_type(qi, field_access(quote!(self), Some(&field.ident), idx));
                    let ty = quote!(Some(#value_type));
                    struct_field(qi, field, ty)
                });
                quote! {
                    #qi ::Type::Tuple(
                        #qi ::ty::Tuple::Struct {
                            name: #name.to_owned(),
                            fields: vec![ #( #fields ),* ],
                        }
                    )
                }
            }
            ContainerData::TupleStruct { name, fields } => {
                let fields = fields
                    .iter()
                    .enumerate()
                    .map(|(idx, _)| value_type(qi, field_access(quote!(self), None, idx)));
                quote! {
                    Some(#qi ::Type::Tuple(
                        #qi ::ty::Tuple::TupleStruct {
                            name: #name.to_owned(),
                            elements: vec![ #( #fields ),* ],
                        }
                    ))
                }
            }
        }
    }

    fn as_value(&self) -> TokenStream {
        let qi = &self.crate_path;
        match &self.data {
            ContainerData::Empty => {
                quote! {
                    #qi ::Value::Unit
                }
            }
            ContainerData::Field { field } => {
                let field_access = field_access(quote!(self), field.ident.as_ref(), 0);
                as_value(qi, field_access)
            }
            ContainerData::Struct { fields, .. } => {
                let field_values = fields.iter().map(|field| {
                    let field_ident = &field.ident;
                    let field_access = quote!(self.#field_ident);
                    as_value(qi, field_access)
                });
                quote! {
                    #qi ::Value::Tuple(
                        vec![
                            #( #field_values ),*
                        ]
                    )
                }
            }
            ContainerData::TupleStruct { fields, .. } => {
                let values = fields
                    .iter()
                    .enumerate()
                    .map(|(idx, _)| as_value(qi, quote!(self.#idx)));
                quote! {
                    #qi ::Value::Tuple(
                        vec![
                            #( #values ),*
                        ]
                    )
                }
            }
        }
    }

    fn qi_from_value(&self, value: &Ident) -> TokenStream {
        fn from_tuple<T>(qi: &Path, name: &Ident, value: &Ident, fields: T) -> TokenStream
        where
            T: ToTokens,
        {
            let err_type_mismatch =
                value_type_mismatch_error(qi, format!("a {name}"), quote!(value_type.to_string()));
            quote! {
                let value_type = #qi ::AsValue::value_type(&#value);
                let make_err_type_mismatch = || #err_type_mismatch;
                let tuple = match #value {
                    #qi ::Value::Tuple(tuple) => tuple,
                    _ => return Err(make_err_type_mismatch()),
                };
                let mut iter = tuple.into_iter();
                #fields
            }
        }

        let qi = &self.crate_path;
        let name = &self.name;
        match &self.data {
            ContainerData::Empty => {
                let err_type_mismatch = value_type_mismatch_error(
                    qi,
                    format!("a {name}"),
                    quote!(#qi ::AsValue::value_type(&#value).to_string()),
                );
                quote! {
                    match #value {
                        #qi ::Value::Unit => Ok(Self),
                        _ => Err(#err_type_mismatch),
                    }
                }
            }
            ContainerData::Field { field } => {
                let result = quote!(#qi ::FromValue::from_value(#value));
                match &field.ident {
                    Some(ident) => {
                        quote! {
                            Ok(Self {
                                #ident: #result?
                            })
                        }
                    }
                    None => {
                        quote! {
                            Ok(Self(#result?))
                        }
                    }
                }
            }
            ContainerData::Struct { fields, .. } => {
                let fields = fields.iter().map(|field| {
                    let ident = &field.ident;
                    let value = quote!(iter.next().ok_or_else(&make_err_type_mismatch)?);
                    quote! {
                        #ident: #qi ::FromValue::from_value(#value)?
                    }
                });
                from_tuple(qi, name, value, quote!(Ok(Self { #( #fields ),* })))
            }
            ContainerData::TupleStruct { fields, .. } => {
                let fields = fields.iter().map(|_| {
                    quote! {{
                        let value = iter.next().ok_or_else(make_err_type_mismatch)?;
                        #qi ::FromValue::from_value(value)?
                    }}
                });
                from_tuple(qi, name, value, quote!(Ok(Self (#( #fields ),*))))
            }
        }
    }

    fn std_into_value(&self, value: &Ident) -> TokenStream {
        let qi = &self.crate_path;
        match &self.data {
            ContainerData::Empty => {
                quote! {
                    #qi ::Value::Unit
                }
            }
            ContainerData::Field { field } => {
                let field_access = field_access(value, field.ident.as_ref(), 0);
                value_from(qi, field_access)
            }
            ContainerData::Struct { fields, .. } => {
                let values = fields.iter().map(|field| {
                    let field_ident = &field.ident;
                    value_from(qi, quote!(#value. #field_ident))
                });
                quote! {
                    #qi ::Value::Tuple(
                        vec![
                            #( #values ),*
                        ]
                    )
                }
            }
            ContainerData::TupleStruct { fields, .. } => {
                let values = fields
                    .iter()
                    .enumerate()
                    .map(|(idx, _)| value_from(qi, quote!(#value. #idx)));
                quote! {
                    #qi ::Value::Tuple(
                        vec![
                            #( #values ),*
                        ]
                    )
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum Derive {
    Reflect,
    AsValue,
    FromValue,
    StdTryFromValue,
    IntoValue,
}

impl std::fmt::Display for Derive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Derive::Reflect => f.write_str("Reflect"),
            Derive::AsValue => f.write_str("AsValue"),
            Derive::FromValue => f.write_str("FromValue"),
            Derive::StdTryFromValue => f.write_str("StdTryFromValue"),
            Derive::IntoValue => f.write_str("IntoValue"),
        }
    }
}

fn qi_value_lifetime() -> Lifetime {
    parse_quote!('__qi_value)
}

fn split_value_lt_generics(
    value_lt: Lifetime,
    generics: &Generics,
) -> (
    ValueFromImplGenerics<'_>,
    syn::TypeGenerics<'_>,
    Option<&syn::WhereClause>,
) {
    let (_, ty_generics, where_clause) = generics.split_for_impl();
    (
        ValueFromImplGenerics { value_lt, generics },
        ty_generics,
        where_clause,
    )
}

struct ValueFromImplGenerics<'a> {
    value_lt: Lifetime,
    generics: &'a Generics,
}

impl<'a> ToTokens for ValueFromImplGenerics<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let value_lt = &self.value_lt;
        let mut generics = self.generics.clone();
        let lifetimes = generics
            .lifetimes()
            .map(|LifetimeParam { lifetime, .. }| lifetime);
        let mut lt_param = LifetimeParam::new(value_lt.clone());
        lt_param.bounds.extend(lifetimes.cloned());
        generics.params.push(GenericParam::Lifetime(lt_param));
        let (impl_generics, _, _) = generics.split_for_impl();
        impl_generics.to_tokens(tokens)
    }
}

struct ContainerAttributes {
    crate_path: Path,
    transparent: bool,
    rename_all: Option<Case>,
}

impl ContainerAttributes {
    /// Parses attributes with syntax:
    /// #[qi(value = "...", transparent, rename_all = "...")].
    fn new(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut crate_path = parse_quote!(::qi_value);
        let mut transparent = false;
        let mut rename_all = None;
        for attr in attrs {
            if attr.path().is_ident("qi") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("value") {
                        let value = meta.value()?; // parses the '='
                        let path_lit_str: LitStr = value.parse()?;
                        crate_path = path_lit_str.parse()?;
                        Ok(())
                    } else if meta.path.is_ident("transparent") {
                        transparent = true;
                        Ok(())
                    } else if meta.path.is_ident("rename_all") {
                        let value = meta.value()?; // parses the '='
                        let value_lit_str: LitStr = value.parse()?;
                        rename_all = Some(match value_lit_str.value().as_str() {
                            "lowercase" => Case::Lower,
                            "UPPERCASE" => Case::Upper,
                            "PascalCase" => Case::Pascal,
                            "camelCase" => Case::Camel,
                            "snake_case" => Case::Snake,
                            "SCREAMING_SNAKE_CASE" => Case::ScreamingSnake,
                            "kebab-case" => Case::Kebab,
                            "SCREAMING-KEBAB-CASE" => Case::UpperKebab,
                            _ => return Err(meta.error("unknown \"rename_all\" value")),
                        });
                        Ok(())
                    } else {
                        Err(meta.error("unknown attribute"))
                    }
                })?;
            }
        }

        Ok(Self {
            crate_path,
            transparent,
            rename_all,
        })
    }
}

enum ContainerData {
    Empty,
    Field {
        field: syn::Field,
    },
    Struct {
        name: String,
        fields: Vec<RenamedField>,
    },
    TupleStruct {
        name: String,
        fields: Vec<Type>,
    },
}

impl ContainerData {
    fn new_struct(name: String, data: DataStruct, rename_all: Option<Case>) -> Self {
        match data.fields {
            Fields::Named(fields) => {
                let fields = fields
                    .named
                    .into_iter()
                    .map(|field| RenamedField {
                        ident: field.ident.unwrap(),
                        rename: rename_all,
                        ty: field.ty,
                    })
                    .collect();
                Self::Struct { name, fields }
            }
            Fields::Unnamed(fields) => {
                let fields = fields.unnamed.into_iter().map(|field| field.ty).collect();
                Self::TupleStruct { name, fields }
            }
            Fields::Unit => Self::TupleStruct {
                name,
                fields: vec![],
            },
        }
    }

    fn new_struct_transparent(data: DataStruct) -> Result<Self, MakeContainerContentError> {
        let field = data.fields.into_iter().try_fold(None, |ty, field| {
            if ty.is_some() {
                Err(MakeContainerContentError::TransparentWithMoreThanOneField)
            } else {
                Ok(Some(field))
            }
        })?;
        Ok(match field {
            Some(field) => Self::Field { field },
            None => Self::Empty,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum MakeContainerContentError {
    #[error("`qi(transparent)` requires struct to have at most one field")]
    TransparentWithMoreThanOneField,
}

struct RenamedField {
    ident: Ident,
    rename: Option<Case>,
    ty: Type,
}

fn reflect_ty<T: ToTokens>(qi: &Path, ty: T) -> TokenStream {
    quote_spanned!(ty.span()=> < #ty as #qi ::Reflect >::ty())
}

fn value_type<T: ToTokens>(qi: &Path, value: T) -> TokenStream {
    quote_spanned!(value.span()=> #qi ::AsValue::value_type(&#value))
}

fn field_access<T: ToTokens>(object: T, field: Option<&Ident>, index: usize) -> TokenStream {
    match field {
        Some(field) => quote!(#object.#field),
        None => {
            let index = Index::from(index);
            quote!(#object.#index)
        }
    }
}

fn struct_field<T: ToTokens>(qi: &Path, field: &RenamedField, ty: T) -> TokenStream {
    let field_name = field.ident.to_string();
    let field_name = match field.rename {
        Some(case) => field_name.to_case(case),
        None => field_name,
    };
    quote! {
        #qi ::ty::StructField {
            name: #field_name .to_owned(),
            ty: #ty,
        }
    }
}

fn as_value<T: ToTokens>(qi: &Path, v: T) -> TokenStream {
    quote!(#qi ::AsValue::as_value(&#v))
}

fn value_type_mismatch_error<T: ToTokens>(qi: &Path, expected: String, actual: T) -> TokenStream {
    quote! {
        #qi ::FromValueError::TypeMismatch {
            expected: #expected.to_owned(),
            actual: #actual,
        }
    }
}

fn value_from<T: ToTokens>(qi: &Path, v: T) -> TokenStream {
    quote!(#qi ::Value::from(#v))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use syn::Expr;

    #[test]
    fn test_reflect_ty() {
        let qi = &parse_quote!(my_value_crate);
        let ty: &Type = &parse_quote!(MyType);
        let reflect_ty = reflect_ty(qi, ty);
        let reflect_ty_expr: Expr = parse_quote!(#reflect_ty);
        assert_eq!(
            reflect_ty_expr,
            parse_quote!(<MyType as my_value_crate::Reflect>::ty())
        );
    }
}
