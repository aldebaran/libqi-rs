use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
    parse_quote, parse_quote_spanned, spanned::Spanned, Attribute, Data, DataStruct, DeriveInput,
    Error, Fields, GenericParam, Generics, Ident, LitStr, Path, Type,
};

pub(crate) fn derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let container = Container::new(input)?;
    Ok(container.quote_derive())
}

struct Container {
    name: Ident,
    crate_path: Path,
    generics: Generics,
    data: ContainerData,
}

impl Container {
    fn new(input: DeriveInput) -> syn::Result<Self> {
        let attrs = ContainerAttributes::new(&input.attrs)?;
        let name = input.ident;
        let crate_path = attrs.crate_path;
        let generics = make_trait_bounds(&crate_path, input.generics);
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
                "`Typed` cannot be derived on enums and unions",
            )),
        }?;
        Ok(Self {
            name,
            crate_path,
            generics,
            data,
        })
    }

    fn quote_derive(self) -> TokenStream {
        let crate_path = &self.crate_path;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let typed_ty_impl = self.data.quote_typed_ty_expr(crate_path);

        let name = &self.name;
        quote! {
            impl #impl_generics #crate_path ::Typed for #name #ty_generics #where_clause {
                fn ty() -> Option<#crate_path ::Type> {
                    #typed_ty_impl
                }
            }
        }
    }
}

struct ContainerAttributes {
    crate_path: Path,
    transparent: bool,
    rename_all: Option<Case>,
}

impl ContainerAttributes {
    /// Parses attributes with syntax:
    /// #[qi(typed(crate = "...", transparent)].
    fn new(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut crate_path = parse_quote!(::qi_type);
        let mut transparent = false;
        let mut rename_all = None;
        for attr in attrs {
            if attr.path().is_ident("qi") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("typed") {
                        meta.parse_nested_meta(|meta| {
                            if meta.path.is_ident("crate") {
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
                    Ok(())
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
    Transparent { ty: Option<Type> },
    Struct { name: String, fields: Vec<Field> },
    TupleStruct { name: String, elements: Vec<Type> },
}

impl ContainerData {
    fn new_struct(name: String, data: DataStruct, rename_all: Option<Case>) -> Self {
        match data.fields {
            Fields::Named(fields) => {
                let fields = fields
                    .named
                    .into_iter()
                    .map(|field| Field {
                        name: field.ident.unwrap().to_string(),
                        rename: rename_all,
                        ty: field.ty,
                    })
                    .collect();
                Self::Struct { name, fields }
            }
            Fields::Unnamed(fields) => {
                let elements = fields.unnamed.into_iter().map(|field| field.ty).collect();
                Self::TupleStruct { name, elements }
            }
            Fields::Unit => Self::TupleStruct {
                name,
                elements: vec![],
            },
        }
    }

    fn new_struct_transparent(data: DataStruct) -> Result<Self, MakeContainerContentError> {
        let ty = data.fields.into_iter().try_fold(None, |ty, field| {
            if ty.is_some() {
                Err(MakeContainerContentError::TransparentWithMoreThanOneField)
            } else {
                Ok(Some(field.ty))
            }
        })?;
        Ok(Self::Transparent { ty })
    }

    fn quote_typed_ty_expr(self, crate_path: &Path) -> TokenStream {
        match self {
            ContainerData::Transparent { ty } => {
                let ty = ty.unwrap_or_else(|| parse_quote!(()));
                quote_typed_ty(crate_path, &ty)
            }
            ContainerData::Struct { name, fields } => {
                let fields = fields
                    .into_iter()
                    .map(|field| struct_field(crate_path, field));
                quote! {
                    Some(#crate_path ::Type::Tuple(
                        #crate_path ::Tuple::Struct {
                            name: #name.to_owned(),
                            fields: vec![ #( #fields ),* ],
                        }
                    ))
                }
            }
            ContainerData::TupleStruct { name, elements } => {
                let elements = elements
                    .into_iter()
                    .map(|element| quote_typed_ty(crate_path, &element));
                quote! {
                    Some(#crate_path ::Type::Tuple(
                        #crate_path ::Tuple::TupleStruct {
                            name: #name.to_owned(),
                            elements: vec![ #( #elements ),* ],
                        }
                    ))
                }
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum MakeContainerContentError {
    #[error("typed(transparent) requires struct to have at most one field")]
    TransparentWithMoreThanOneField,
}

struct Field {
    name: String,
    rename: Option<Case>,
    ty: Type,
}

// Makes a bound `T: Typed` for every type parameter T.
fn make_trait_bounds(crate_path: &Path, mut generics: Generics) -> Generics {
    let span = generics.span();
    for param in &mut generics.params {
        if let GenericParam::Type(type_param) = param {
            type_param.bounds.push(parse_quote_spanned! {span=>
                #crate_path ::Typed
            });
        }
    }
    generics
}

fn quote_typed_ty(crate_path: &Path, ty: &Type) -> TokenStream {
    quote_spanned!(ty.span()=> < #ty as #crate_path ::Typed >::ty())
}

fn struct_field(crate_path: &Path, field: Field) -> TokenStream {
    let field_name = field.name.to_string();
    let field_name = match field.rename {
        Some(case) => field_name.to_case(case),
        None => field_name,
    };
    let field_ty = quote_typed_ty(crate_path, &field.ty);
    quote! {
        #crate_path ::StructField {
            name: #field_name .to_owned(),
            ty: #field_ty,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use syn::{parse2, Expr};

    #[test]
    fn test_quote_typed_ty() {
        let crate_path = &parse_quote!(my_typed_crate);
        let ty = &parse_quote!(MyType);
        assert_eq!(
            parse2::<Expr>(quote_typed_ty(crate_path, ty)).unwrap(),
            parse_quote!(<MyType as my_typed_crate::Typed>::ty())
        );
    }

    #[test]
    fn test_make_traits_bounds() {
        let crate_path = &parse_quote!(my_typed_crate);
        let generics = parse_quote!(<T:, U: ToString>);
        assert_eq!(
            make_trait_bounds(crate_path, generics),
            parse_quote!(<T: my_typed_crate::Typed, U: ToString + my_typed_crate::Typed>)
        );
    }
}
