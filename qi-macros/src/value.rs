use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse_quote, Attribute, Data, DataStruct, DeriveInput, Error, Fields, GenericParam, Generics,
    Ident, Lifetime, LifetimeParam, LitStr, Path, TypeParamBound,
};

pub(crate) fn derive_impl(derive: Trait, input: DeriveInput) -> syn::Result<TokenStream> {
    Ok(Derive::new(derive, input)?.derive_impl())
}

pub(crate) struct Derive {
    derive_trait: Trait,
    name: Ident,
    crate_path: Path,
    generics: Generics,
    data: ContainerData,
}

impl Derive {
    pub(crate) fn new(derive_trait: Trait, input: DeriveInput) -> syn::Result<Self> {
        let attrs = DeriveAttributes::new(&input.attrs)?;
        let name = input.ident;
        let crate_path = attrs.crate_path;
        let generics = input.generics;
        let data = match input.data {
            Data::Struct(data) => {
                if attrs.transparent {
                    ContainerData::new_struct_transparent(&name, data)
                } else {
                    ContainerData::new_struct(data, attrs.rename_all)
                }
            }
            Data::Enum(_) | Data::Union(_) => Err(Error::new_spanned(
                &name,
                format!("{} cannot be derived on enums and unions", derive_trait),
            )),
        }?;
        Ok(Self {
            derive_trait,
            name,
            crate_path,
            generics,
            data,
        })
    }

    fn derive_impl(&self) -> TokenStream {
        match self.derive_trait {
            Trait::Valuable => {
                let reflect = self.impl_reflect();
                let from_value = self.impl_from_value();
                let to_value = self.impl_to_value();
                let into_value = self.impl_into_value();
                quote! {
                    #reflect
                    #from_value
                    #to_value
                    #into_value
                }
            }

            Trait::Reflect => self.impl_reflect(),
            Trait::ToValue => self.impl_to_value(),
            Trait::FromValue => self.impl_from_value(),
            Trait::IntoValue => self.impl_into_value(),
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
        let reflect_ty = self.reflect_ty(Reflect::Static);
        let rt_reflect_ty = self.reflect_ty(Reflect::Runtime);

        quote! {
            impl #impl_generics #qi ::Reflect for #name #ty_generics #where_clause {
                fn ty() -> Option<#qi ::Type> {
                    #reflect_ty
                }
            }

            impl #impl_generics #qi ::RuntimeReflect for #name #ty_generics #where_clause {
                fn ty(&self) -> #qi ::Type {
                    #rt_reflect_ty
                }
            }
        }
    }

    fn impl_to_value(&self) -> TokenStream {
        let qi = &self.crate_path;
        let name = &self.name;

        let mut generics = self.generics.clone();

        // Add a bound `T: ToValue` for each type parameter T.
        let to_value_bound: TypeParamBound = parse_quote!(#qi ::ToValue);
        for param in generics.type_params_mut() {
            param.bounds.push(to_value_bound.clone());
        }

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let to_value = self.to_value();

        quote! {
            impl #impl_generics #qi ::ToValue for #name #ty_generics #where_clause {
                fn to_value(&self) -> #qi:: Value<'_> {
                    #to_value
                }
            }
        }
    }

    fn impl_from_value(&self) -> TokenStream {
        let qi = &self.crate_path;
        let name = &self.name;

        let value_lt = qi_value_lifetime();

        let mut generics = self.generics.clone();

        // Add a bound `T: FromValue` for each type parameter T.
        for ty in generics.type_params_mut() {
            ty.bounds.push(parse_quote!(#qi ::FromValue<#value_lt>));
        }

        // Add a bound `'value: 'a` for each lifetime parameter `'a``.
        let lifetime_params = generics
            .lifetimes()
            .map(|lt| lt.lifetime.clone())
            .collect::<Vec<_>>();
        let where_clause = generics.make_where_clause();
        where_clause
            .predicates
            .push(parse_quote!(#value_lt: #(#lifetime_params)+*));

        let (impl_generics, ty_generics, where_clause) =
            split_generics_with_value_lt_impl(value_lt.clone(), &generics);

        let value_ident = parse_quote!(value);
        let from_value = self.from_value(&value_ident);
        quote! {
            impl #impl_generics #qi ::FromValue<#value_lt> for #name #ty_generics #where_clause {
                fn from_value(#value_ident: #qi ::Value<#value_lt>) -> std::result::Result<Self, #qi ::FromValueError> {
                    #from_value
                }
            }
        }
    }

    fn impl_into_value(&self) -> TokenStream {
        let qi = &self.crate_path;
        let name = &self.name;
        let value_lt = qi_value_lifetime();

        let mut generics = self.generics.clone();

        // Add a bound `'a: 'value` for each lifetime parameter 'a.
        for lt in generics.lifetimes_mut() {
            lt.bounds.push(value_lt.clone());
        }

        // Add a bound `T: IntoValue` for each type parameter T.
        for ty in generics.type_params_mut() {
            ty.bounds.push(parse_quote!(#qi IntoValue< #value_lt >));
        }

        let (impl_generics, ty_generics, where_clause) =
            split_generics_with_value_lt_impl(value_lt.clone(), &generics);

        let into_value = self.into_value();
        quote! {
            impl #impl_generics #qi ::IntoValue< #value_lt > for #name #ty_generics #where_clause {
                fn into_value(self) -> #qi ::Value< #value_lt > {
                    #into_value
                }
            }
        }
    }

    fn reflect_ty(&self, reflect: Reflect) -> TokenStream {
        let name_str = self.name.to_string();
        let qi = &self.crate_path;
        match &self.data {
            ContainerData::Empty => reflect.convert_to_ty_result(quote!(#qi ::Type::Unit)),
            ContainerData::Field(field) => field.reflect_ty(qi, reflect),
            ContainerData::Struct { fields } => {
                let fields = fields.iter().map(|field| field.struct_field(qi, reflect));
                reflect.convert_to_ty_result(quote! {
                    #qi ::Type::Tuple(
                        #qi ::ty::Tuple::Struct {
                            name: #name_str.to_owned(),
                            fields: vec![ #( #fields ),* ],
                        }
                    )
                })
            }
            ContainerData::TupleStruct { fields } => {
                let fields = fields.iter().map(|field| {
                    reflect.convert_ty_result_to_option(field.reflect_ty(qi, reflect))
                });
                reflect.convert_to_ty_result(quote! {
                    #qi ::Type::Tuple(
                        #qi ::ty::Tuple::TupleStruct {
                            name: #name_str.to_owned(),
                            elements: vec![ #( #fields ),* ],
                        }
                    )
                })
            }
        }
    }

    fn to_value(&self) -> TokenStream {
        let qi = &self.crate_path;
        match &self.data {
            ContainerData::Empty => {
                quote! {
                    #qi ::Value::Unit
                }
            }
            ContainerData::Field(field) => field.to_value(qi),
            ContainerData::Struct { fields, .. } | ContainerData::TupleStruct { fields, .. } => {
                let fields = fields.iter().map(|field| field.to_value(qi));
                quote! {
                    #qi ::Value::Tuple(
                        vec![
                            #( #fields ),*
                        ]
                    )
                }
            }
        }
    }

    fn into_value(&self) -> TokenStream {
        let qi = &self.crate_path;
        match &self.data {
            ContainerData::Empty => {
                quote! {
                    #qi ::Value::Unit
                }
            }
            ContainerData::Field(field) => field.into_value(qi),
            ContainerData::Struct { fields, .. } | ContainerData::TupleStruct { fields } => {
                let values = fields.iter().map(|field| field.into_value(qi));
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

    fn from_value(&self, value: &Ident) -> TokenStream {
        fn from_tuple<F, U>(qi: &Path, name: String, value: &Ident, struct_data: F) -> TokenStream
        where
            F: FnOnce(&TokenStream) -> U,
            U: ToTokens,
        {
            let err_type_mismatch =
                value_type_mismatch_error(qi, quote!(#name.to_owned()), quote!(#value.to_string()));
            let err = quote!(err);
            let struct_data = struct_data(&err);
            quote! {
                let #err = #err_type_mismatch;
                match #value {
                    #qi ::Value::Tuple(tuple) => {
                        let mut iter = tuple.into_iter();
                        Ok(Self #struct_data)
                    }
                    _ => Err(#err),
                }
            }
        }

        let qi = &self.crate_path;
        let name = &self.name;
        let name_str = name.to_string();
        match &self.data {
            ContainerData::Empty => {
                let err = value_type_mismatch_error(
                    qi,
                    quote!(#name_str.to_owned()),
                    quote!(#value.to_string()),
                );
                quote! {
                    match #value {
                        #qi ::Value::Unit => Ok(Self),
                        _ => Err(#err),
                    }
                }
            }
            ContainerData::Field(field) => {
                let from_value = field.from_value(qi, quote!(value));
                match &field.src.ident {
                    Some(ident) => {
                        quote! {
                            Ok(Self {
                                #ident: #from_value?
                            })
                        }
                    }
                    None => {
                        quote! {
                            Ok(Self(#from_value?))
                        }
                    }
                }
            }
            ContainerData::Struct { fields, .. } => from_tuple(qi, name_str, value, |err| {
                let fields = fields.iter().map(|field| {
                    let ident = &field.src.ident;
                    let from_value = field.from_value(qi, quote!(value));
                    quote! {
                        #ident: match iter.next() {
                            Some(value) => #from_value?,
                            None => return Err(#err),
                        }
                    }
                });
                quote!({ #( #fields ),* })
            }),
            ContainerData::TupleStruct { fields, .. } => from_tuple(qi, name_str, value, |err| {
                let fields = fields.iter().map(|field| {
                    let from_value = field.from_value(qi, quote!(value));
                    quote! {
                        match iter.next() {
                            Some(value) => #from_value?,
                            None => return Err(#err),
                        }
                    }
                });
                quote!(( #( #fields ),* ))
            }),
        }
    }
}

#[derive(Debug)]
pub(crate) enum Trait {
    Valuable,
    Reflect,
    ToValue,
    IntoValue,
    FromValue,
}

impl std::fmt::Display for Trait {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Trait::Valuable => f.write_str("Value"),
            Trait::Reflect => f.write_str("Reflect"),
            Trait::ToValue => f.write_str("ToValue"),
            Trait::FromValue => f.write_str("FromValue"),
            Trait::IntoValue => f.write_str("IntoValue"),
        }
    }
}

fn qi_value_lifetime() -> Lifetime {
    Lifetime::new("'__qi_value", Span::call_site())
}

fn split_generics_with_value_lt_impl(
    value_lt: Lifetime,
    generics: &Generics,
) -> (
    WithValueLifetimeImplGenerics<'_>,
    syn::TypeGenerics<'_>,
    Option<&syn::WhereClause>,
) {
    let (_, ty_generics, where_clause) = generics.split_for_impl();
    (
        WithValueLifetimeImplGenerics { value_lt, generics },
        ty_generics,
        where_clause,
    )
}

struct WithValueLifetimeImplGenerics<'a> {
    value_lt: Lifetime,
    generics: &'a Generics,
}

impl<'a> ToTokens for WithValueLifetimeImplGenerics<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut generics = self.generics.clone();
        generics
            .params
            .push(GenericParam::Lifetime(LifetimeParam::new(
                self.value_lt.clone(),
            )));
        let (impl_generics, _, _) = generics.split_for_impl();
        impl_generics.to_tokens(tokens)
    }
}

struct DeriveAttributes {
    crate_path: Path,
    transparent: bool,
    rename_all: Option<Case>,
}

impl DeriveAttributes {
    /// Parses attributes with syntax:
    /// #[qi(value = "...", transparent, rename_all = "...")].
    fn new(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut crate_path = parse_quote!(::qi::value);
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
                        rename_all = Some(parse_rename_attribute(&meta)?);
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
    Field(Field),
    Struct { fields: Vec<Field> },
    TupleStruct { fields: Vec<Field> },
}

impl ContainerData {
    fn new_struct(data: DataStruct, rename_all: Option<Case>) -> syn::Result<Self> {
        Ok(match data.fields {
            Fields::Named(fields) => {
                let fields = fields
                    .named
                    .into_iter()
                    .enumerate()
                    .map(|(idx, field)| Field::new(field, idx, rename_all))
                    .collect::<syn::Result<_>>()?;
                Self::Struct { fields }
            }
            Fields::Unnamed(fields) => {
                let fields = fields
                    .unnamed
                    .into_iter()
                    .enumerate()
                    .map(|(idx, field)| Field::new(field, idx, rename_all))
                    .collect::<syn::Result<_>>()?;
                Self::TupleStruct { fields }
            }
            Fields::Unit => Self::TupleStruct { fields: vec![] },
        })
    }

    fn new_struct_transparent(name: &Ident, data: DataStruct) -> syn::Result<Self> {
        let mut field_iter = data.fields.into_iter();
        if field_iter.len() > 1 {
            return Err(syn::Error::new_spanned(
                name,
                "`qi(transparent)` requires struct to have at most one field",
            ));
        }
        Ok(match field_iter.next() {
            Some(field) => Self::Field(Field::new(field, 0, None)?),
            None => Self::Empty,
        })
    }
}

struct Field {
    src: syn::Field,
    index: syn::Index,
    attrs: FieldAttributes,
}

impl Field {
    fn new(src: syn::Field, index: usize, rename: Option<Case>) -> syn::Result<Self> {
        let index = syn::Index::from(index);
        let attrs = FieldAttributes::new(rename, &src.attrs)?;
        Ok(Self { src, index, attrs })
    }

    fn reflect_ty(&self, qi: &Path, reflect: Reflect) -> TokenStream {
        let ty = &self.src.ty;
        match (reflect, self.attrs.as_raw) {
            (Reflect::Static, false) => Reflect::static_ty(qi, ty),
            (Reflect::Static, true) => Reflect::static_ty(qi, quote!(#qi ::AsRaw<#ty>)),
            (Reflect::Runtime, false) => Reflect::runtime_ty(qi, self.access_on(quote!(self))),
            (Reflect::Runtime, true) => {
                let access = self.access_on(quote!(self));
                Reflect::runtime_ty(qi, quote!(#qi ::AsRaw(&#access)))
            }
        }
    }

    fn struct_field(&self, qi: &Path, reflect: Reflect) -> TokenStream {
        let field_name = self.name();
        let ty = reflect.convert_ty_result_to_option(self.reflect_ty(qi, reflect));
        quote! {
            #qi ::ty::StructField {
                name: #field_name .to_owned(),
                ty: #ty,
            }
        }
    }

    fn access_on<T>(&self, receiver: T) -> TokenStream
    where
        T: ToTokens,
    {
        match &self.src.ident {
            Some(ident) => quote!(#receiver.#ident),
            None => {
                let index = &self.index;
                quote!(#receiver.#index)
            }
        }
    }

    fn name(&self) -> Option<String> {
        self.src.ident.as_ref().map(|ident| {
            let name = ident.to_string();
            match self.attrs.rename {
                Some(case) => name.to_case(case),
                None => name,
            }
        })
    }

    fn into_value(&self, qi: &Path) -> TokenStream {
        let mut value = self.access_on(quote!(self));
        if self.attrs.as_raw {
            value = quote!(#qi:: AsRaw(#value));
        }
        quote!(#qi ::IntoValue::into_value(#value))
    }

    fn to_value(&self, qi: &Path) -> TokenStream {
        let value = self.access_on(quote!(self));
        let value_ref = quote!(&#value);
        if self.attrs.as_raw {
            quote!(#qi ::IntoValue::into_value(#qi:: AsRaw(#value_ref)))
        } else {
            quote!(#qi ::ToValue::to_value(#value_ref))
        }
    }

    fn from_value<T>(&self, qi: &Path, value: T) -> TokenStream
    where
        T: ToTokens,
    {
        let mut result = quote!(#qi ::FromValue::from_value(#value));
        if self.attrs.as_raw {
            result = quote!(#result .map(|#qi ::AsRaw(value)| value));
        }
        result
    }
}

struct FieldAttributes {
    as_raw: bool,
    rename: Option<Case>,
}

impl FieldAttributes {
    /// Parses attributes with syntax:
    /// #[qi(rename = "...", as_raw)].
    fn new(mut rename: Option<Case>, attrs: &[Attribute]) -> syn::Result<Self> {
        let mut as_raw = false;
        for attr in attrs {
            if attr.path().is_ident("qi") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename") {
                        rename = Some(parse_rename_attribute(&meta)?);
                        Ok(())
                    } else if meta.path.is_ident("as_raw") {
                        as_raw = true;
                        Ok(())
                    } else {
                        Err(meta.error("unknown attribute"))
                    }
                })?;
            }
        }
        Ok(Self { as_raw, rename })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Reflect {
    Static,
    Runtime,
}

impl Reflect {
    fn static_ty<T>(qi: &Path, ty: T) -> TokenStream
    where
        T: ToTokens,
    {
        quote!(< #ty as #qi ::Reflect >::ty())
    }

    fn runtime_ty<T>(qi: &Path, value: T) -> TokenStream
    where
        T: ToTokens,
    {
        quote!(#qi ::RuntimeReflect::ty(&#value))
    }

    fn convert_to_ty_result<T>(self, value: T) -> TokenStream
    where
        T: ToTokens,
    {
        match self {
            Reflect::Static => some(value),
            Reflect::Runtime => value.into_token_stream(),
        }
    }

    fn convert_ty_result_to_option<T>(self, value: T) -> TokenStream
    where
        T: ToTokens,
    {
        match self {
            Reflect::Static => value.into_token_stream(),
            Reflect::Runtime => some(value),
        }
    }
}

fn parse_rename_attribute(meta: &syn::meta::ParseNestedMeta) -> syn::Result<Case> {
    let value = meta.value()?; // parses the '='
    let value_lit_str: LitStr = value.parse()?;
    match value_lit_str.value().as_str() {
        "lowercase" => Ok(Case::Lower),
        "UPPERCASE" => Ok(Case::Upper),
        "PascalCase" => Ok(Case::Pascal),
        "camelCase" => Ok(Case::Camel),
        "snake_case" => Ok(Case::Snake),
        "SCREAMING_SNAKE_CASE" => Ok(Case::ScreamingSnake),
        "kebab-case" => Ok(Case::Kebab),
        "SCREAMING-KEBAB-CASE" => Ok(Case::UpperKebab),
        _ => Err(meta.error("unknown \"rename_all\" value")),
    }
}

impl ToTokens for Field {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.src.to_tokens(tokens)
    }
}

fn some<T>(value: T) -> TokenStream
where
    T: ToTokens,
{
    quote!(Some(#value))
}

fn value_type_mismatch_error(
    qi: &Path,
    expected: impl ToTokens,
    actual: impl ToTokens,
) -> TokenStream {
    quote! {
        #qi ::FromValueError::TypeMismatch {
            expected: #expected,
            actual: #actual,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use syn::{Expr, Type};

    #[test]
    fn test_reflect_ty() {
        let qi = &parse_quote!(my_value_crate);
        let ty: &Type = &parse_quote!(MyType);
        let reflect_ty = Reflect::static_ty(qi, ty);
        let reflect_ty_expr: Expr = parse_quote!(#reflect_ty);
        assert_eq!(
            reflect_ty_expr,
            parse_quote!(<MyType as my_value_crate::Reflect>::ty())
        );
    }
}
