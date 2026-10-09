use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    AttrStyle, Attribute, Error, Expr, ExprLit, FnArg, Ident, ItemTrait, Lit, LitStr, Meta,
    MetaNameValue, Pat, Result, ReturnType, TraitItem, TraitItemFn, Type,
};

pub(super) struct Object {
    trait_item: ItemTrait,
    members: Vec<Member>,
    pub(super) mode: ObjectMode,
}

/// `client` implements `Object` for the generated client only.
///
/// The default mode emits `impl<T: Trait> Object for T`. A crate can contain
/// one such impl, so a second object trait uses `client` instead.
pub(super) enum ObjectMode {
    Blanket,
    Client,
}

impl Parse for ObjectMode {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Ok(Self::Blanket);
        }
        let ident: Ident = input.parse()?;
        if ident != "client" {
            return Err(Error::new(ident.span(), "expected `client`"));
        }
        if input.is_empty() {
            Ok(Self::Client)
        } else {
            Err(Error::new(input.span(), "unexpected tokens after `client`"))
        }
    }
}

impl ToTokens for Object {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let trait_item = &self.trait_item;
        tokens.extend(quote! {
            #[::async_trait::async_trait]
            #trait_item
        });
        self.meta_object().to_tokens(tokens);
        match self.mode {
            ObjectMode::Blanket => self.object_impl().to_tokens(tokens),
            ObjectMode::Client => self.client_object_impl().to_tokens(tokens),
        }
        self.client_impl().to_tokens(tokens);
    }
}

impl Object {
    fn meta_object(&self) -> TokenStream {
        let vis = &self.trait_item.vis;
        let ident = meta_object_ident(&self.trait_item.ident);
        let members = self.members.iter().map(Member::to_add_tokens);
        quote! {
            #vis static #ident: ::once_cell::sync::Lazy<::qi::object::MetaObject> =
                ::once_cell::sync::Lazy::new(|| {
                    let mut builder = ::qi::object::MetaObject::builder();
                    let mut action_id = ::qi::object::ACTION_START_ID;
                    #(#members)*
                    builder.build()
                });
        }
    }

    // Default mode emits `impl<T: Trait> Object for T`, so a crate can
    // define only one such trait. `#[qi::object(client)]` does not emit it.
    fn object_impl(&self) -> TokenStream {
        let trait_ident = &self.trait_item.ident;
        let meta_ident = meta_object_ident(trait_ident);
        let method_arms = self.members.iter().filter_map(|member| match member {
            Member::Method(method) => Some(method.to_call_arm()),
            Member::Signal(_) | Member::Property(_) => None,
        });
        let property_get_arms = self.members.iter().filter_map(|member| match member {
            Member::Property(property) => Some(property.to_get_arm()),
            Member::Method(_) | Member::Signal(_) => None,
        });
        let property_set_arms = self.members.iter().filter_map(|member| match member {
            Member::Property(property) => Some(property.to_set_arm()),
            Member::Method(_) | Member::Signal(_) => None,
        });
        let signal_arms = self.members.iter().filter_map(|member| match member {
            Member::Signal(signal) => Some(signal.to_event_arm()),
            Member::Method(_) | Member::Property(_) => None,
        });
        quote! {
            #[::async_trait::async_trait]
            impl<T> ::qi::Object for T
            where
                T: #trait_ident + ::std::marker::Send + ::std::marker::Sync,
            {
                fn meta(&self) -> &::qi::object::MetaObject {
                    &*#meta_ident
                }

                async fn meta_call(
                    &self,
                    ident: ::qi::object::MemberIdent,
                    args: ::qi::Value<'_>,
                ) -> ::std::result::Result<::qi::Value<'static>, ::qi::Error> {
                    if ident == ::qi::object::MemberIdent::Id(::qi::object::ACTION_ID_PROPERTY) {
                        let prop_ident: ::qi::value::Dynamic<::qi::object::MemberIdent> = args
                            .cast_into()
                            .map_err(|err| ::qi::Error::from(::qi::BoxError::from(err)))?;
                        let prop_ident = prop_ident.into_inner();
                        let property = #meta_ident.property(&prop_ident).ok_or_else(|| {
                            ::qi::Error::MethodNotFound(prop_ident)
                        })?;
                        return match property.name.as_str() {
                            #(#property_get_arms)*
                            _ => ::std::result::Result::Err(::qi::Error::MethodNotFound(
                                ::qi::object::MemberIdent::from(property.name.clone()),
                            )),
                        };
                    }
                    if ident == ::qi::object::MemberIdent::Id(::qi::object::ACTION_ID_SET_PROPERTY)
                    {
                        let (prop_ident, raw_value): (
                            ::qi::value::Dynamic<::qi::object::MemberIdent>,
                            ::qi::value::Dynamic<::qi::Value<'_>>,
                        ) = args
                            .cast_into()
                            .map_err(|err| ::qi::Error::from(::qi::BoxError::from(err)))?;
                        let prop_ident = prop_ident.into_inner();
                        let raw_value = raw_value.into_inner();
                        let property = #meta_ident.property(&prop_ident).ok_or_else(|| {
                            ::qi::Error::MethodNotFound(prop_ident)
                        })?;
                        return match property.name.as_str() {
                            #(#property_set_arms)*
                            _ => {
                                let _ = raw_value;
                                ::std::result::Result::Err(::qi::Error::MethodNotFound(
                                    ::qi::object::MemberIdent::from(property.name.clone()),
                                ))
                            }
                        };
                    }
                    let method = #meta_ident
                        .method(&ident)
                        .ok_or_else(|| ::qi::Error::MethodNotFound(ident))?;
                    match method.name.as_str() {
                        #(#method_arms)*
                        _ => {
                            let _ = args;
                            ::std::result::Result::Err(::qi::Error::MethodNotFound(
                                ::qi::object::MemberIdent::from(method.name.clone()),
                            ))
                        }
                    }
                }

                async fn meta_post(
                    &self,
                    ident: ::qi::object::MemberIdent,
                    args: ::qi::Value<'_>,
                ) {
                    let _res = self.meta_call(ident, args).await;
                }

                async fn meta_event(
                    &self,
                    ident: ::qi::object::MemberIdent,
                    value: ::qi::Value<'_>,
                ) {
                    let Some(signal) = #meta_ident.signal(&ident) else {
                        return;
                    };
                    match signal.name.as_str() {
                        #(#signal_arms)*
                        _ => {
                            let _ = value;
                        }
                    }
                }
            }
        }
    }

    fn client_object_impl(&self) -> TokenStream {
        let client_ident = client_ident(&self.trait_item.ident);
        quote! {
            #[::async_trait::async_trait]
            impl ::qi::Object for #client_ident {
                fn meta(&self) -> &::qi::object::MetaObject {
                    ::qi::Object::meta(&self.client)
                }

                async fn meta_call(
                    &self,
                    ident: ::qi::object::MemberIdent,
                    args: ::qi::Value<'_>,
                ) -> ::std::result::Result<::qi::Value<'static>, ::qi::Error> {
                    ::qi::Object::meta_call(&self.client, ident, args).await
                }

                async fn meta_post(
                    &self,
                    ident: ::qi::object::MemberIdent,
                    args: ::qi::Value<'_>,
                ) {
                    ::qi::Object::meta_post(&self.client, ident, args).await
                }

                async fn meta_event(
                    &self,
                    ident: ::qi::object::MemberIdent,
                    value: ::qi::Value<'_>,
                ) {
                    ::qi::Object::meta_event(&self.client, ident, value).await
                }

                fn uid(&self) -> ::qi::object::Uid {
                    ::qi::Object::uid(&self.client)
                }
            }
        }
    }

    fn client_impl(&self) -> TokenStream {
        let vis = &self.trait_item.vis;
        let trait_ident = &self.trait_item.ident;
        let client_ident = client_ident(trait_ident);
        let checks = self.members.iter().map(Member::to_check_tokens);
        let items = self.members.iter().map(Member::to_client_item);
        quote! {
            #[derive(Clone, Debug)]
            #vis struct #client_ident {
                client: ::qi::ObjectClient,
            }

            impl ::std::convert::TryFrom<::qi::ObjectClient> for #client_ident {
                type Error = ::qi::Error;

                fn try_from(
                    client: ::qi::ObjectClient,
                ) -> ::std::result::Result<Self, ::qi::Error> {
                    let meta = client.meta();
                    #(#checks)*
                    ::std::result::Result::Ok(Self { client })
                }
            }

            #[::async_trait::async_trait]
            impl #trait_ident for #client_ident {
                #(#items)*
            }
        }
    }
}

impl Parse for Object {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut trait_item = ItemTrait::parse(input)?;

        if !trait_item.generics.params.is_empty() {
            return Err(Error::new_spanned(
                &trait_item.generics,
                "qi object traits cannot be generic",
            ));
        }

        let mut members = Vec::with_capacity(trait_item.items.len());
        for item in &mut trait_item.items {
            reject_unknown_qi_attrs(item_attrs(item))?;
            reject_tagged_non_fn(item)?;
            if let Some(method) = Method::from_item(item)? {
                members.push(Member::Method(method));
            } else if let Some(signal) = Signal::from_item(item)? {
                members.push(Member::Signal(signal));
            } else if let Some(property) = Property::from_item(item)? {
                members.push(Member::Property(property));
            }
            strip_qi_attrs(item);
        }

        Ok(Self {
            trait_item,
            members,
            mode: ObjectMode::Blanket,
        })
    }
}

enum Member {
    Method(Method),
    Signal(Signal),
    Property(Property),
}

impl Member {
    fn to_add_tokens(&self) -> TokenStream {
        match self {
            Self::Method(method) => method.to_add_tokens(),
            Self::Signal(signal) => signal.to_add_tokens(),
            Self::Property(property) => property.to_add_tokens(),
        }
    }

    fn to_check_tokens(&self) -> TokenStream {
        match self {
            Self::Method(method) => method.to_check_tokens(),
            Self::Signal(signal) => signal.to_check_tokens(),
            Self::Property(property) => property.to_check_tokens(),
        }
    }

    fn to_client_item(&self) -> TokenStream {
        match self {
            Self::Method(method) => method.to_client_item(),
            Self::Signal(signal) => signal.to_client_item(),
            Self::Property(property) => property.to_client_item(),
        }
    }
}

struct Method {
    func: TraitItemFn,
    name: String,
}

impl Method {
    fn from_item(item: &TraitItem) -> Result<Option<Self>> {
        tagged_fn(item, "qi::method").map(|tagged| tagged.map(|(func, name)| Self { func, name }))
    }

    fn to_add_tokens(&self) -> TokenStream {
        let name = &self.name;
        let description = member_docs(&self.func);
        let set_description = description.map(|description| {
            quote! {
                method.set_description(#description);
            }
        });
        let parameters = method_parameters(&self.func);
        let return_ty = method_return_type(&self.func);
        quote! {
            builder.add_method({
                let uid = action_id.wrapping_next();
                let mut method = ::qi::object::MetaMethod::builder(uid);
                method.set_name(#name);
                #set_description
                #parameters
                method.return_value().set_type(<#return_ty as ::qi::value::Reflect>::ty());
                method.build()
            });
        }
    }

    fn to_call_arm(&self) -> TokenStream {
        let name = &self.name;
        let ident = &self.func.sig.ident;
        let arg_idents = method_arg_idents(&self.func);
        let unpack = match arg_idents.len() {
            0 => quote!(()),
            1 => {
                let arg = &arg_idents[0];
                quote!(#arg)
            }
            _ => quote!((#(#arg_idents),*)),
        };
        let call = if self.func.sig.asyncness.is_some() {
            quote! { self.#ident(#(#arg_idents),*).await }
        } else {
            quote! { self.#ident(#(#arg_idents),*) }
        };
        let value = match &self.func.sig.output {
            ReturnType::Type(_, ty) if is_result(ty) => quote! {
                #call.map_err(::std::convert::Into::<::qi::Error>::into)?
            },
            _ => call,
        };
        quote! {
            #name => {
                let #unpack = args
                    .cast_into()
                    .map_err(|err| ::qi::Error::from(::qi::BoxError::from(err)))?;
                ::std::result::Result::Ok(::qi::value::IntoValue::into_value(#value))
            }
        }
    }

    fn to_check_tokens(&self) -> TokenStream {
        let name = &self.name;
        let parameters = parameters_signature(&self.func);
        let return_ty = method_return_type(&self.func);
        let mismatch = meta_mismatch(name);
        quote! {
            {
                let ident = ::qi::object::MemberIdent::from(#name);
                let method = meta.method(&ident).ok_or_else(|| {
                    ::qi::Error::MethodNotFound(ident)
                })?;
                if method.parameters_signature != #parameters
                    || method.return_signature != <#return_ty as ::qi::value::Reflect>::signature()
                {
                    return ::std::result::Result::Err(#mismatch);
                }
            }
        }
    }

    fn to_client_item(&self) -> TokenStream {
        let sig = &self.func.sig;
        let name = &self.name;
        let arg_idents = method_arg_idents(&self.func);
        let call_args = match arg_idents.len() {
            0 => quote!(()),
            1 => {
                let arg = &arg_idents[0];
                quote!(#arg)
            }
            _ => quote!((#(#arg_idents),*)),
        };
        let return_ty = method_return_type(&self.func);
        let call = quote! {
            ::qi::ObjectExt::call::<#return_ty, _, _>(&self.client, #name, #call_args)
        };
        let polled = if self.func.sig.asyncness.is_some() {
            quote!(#call.await)
        } else {
            quote!(::qi::object::block_on(#call))
        };
        let body = match &self.func.sig.output {
            ReturnType::Type(_, ty) if is_result(ty) => {
                quote!(#polled.map_err(::std::convert::Into::into))
            }
            _ => quote!(#polled.expect("remote call failed")),
        };
        quote! {
            #sig {
                #body
            }
        }
    }
}

struct Signal {
    func: TraitItemFn,
    name: String,
}

impl Signal {
    fn from_item(item: &TraitItem) -> Result<Option<Self>> {
        tagged_fn(item, "qi::signal").map(|tagged| tagged.map(|(func, name)| Self { func, name }))
    }

    fn to_add_tokens(&self) -> TokenStream {
        let name = &self.name;
        let ty = payload_type(&self.func);
        quote! {
            builder.add_signal({
                let uid = action_id.wrapping_next();
                ::qi::object::MetaSignal {
                    uid,
                    name: ::std::string::String::from(#name),
                    signature: <#ty as ::qi::value::Reflect>::signature(),
                }
            });
        }
    }

    fn to_event_arm(&self) -> TokenStream {
        let name = &self.name;
        let call = handle_call(&self.func);
        let ty = payload_type(&self.func);
        quote! {
            #name => {
                if let ::std::result::Result::Ok(value) =
                    ::qi::value::Value::cast_into::<#ty>(value)
                {
                    #call.emit(value);
                }
            }
        }
    }

    fn to_check_tokens(&self) -> TokenStream {
        let name = &self.name;
        let ty = payload_type(&self.func);
        let mismatch = meta_mismatch(name);
        quote! {
            {
                let ident = ::qi::object::MemberIdent::from(#name);
                let signal = meta.signal(&ident).ok_or_else(|| {
                    ::qi::Error::MethodNotFound(ident)
                })?;
                if signal.signature != <#ty as ::qi::value::Reflect>::signature() {
                    return ::std::result::Result::Err(#mismatch);
                }
            }
        }
    }

    fn to_client_item(&self) -> TokenStream {
        let sig = &self.func.sig;
        let name = &self.name;
        quote! {
            #sig {
                ::qi::Signal::remote(self.client.clone(), #name)
            }
        }
    }
}

struct Property {
    func: TraitItemFn,
    name: String,
}

impl Property {
    fn from_item(item: &TraitItem) -> Result<Option<Self>> {
        tagged_fn(item, "qi::property").map(|tagged| tagged.map(|(func, name)| Self { func, name }))
    }

    fn to_add_tokens(&self) -> TokenStream {
        let name = &self.name;
        let ty = payload_type(&self.func);
        quote! {
            builder.add_property({
                let uid = action_id.wrapping_next();
                ::qi::object::MetaProperty {
                    uid,
                    name: ::std::string::String::from(#name),
                    signature: <#ty as ::qi::value::Reflect>::signature(),
                }
            });
        }
    }

    fn to_get_arm(&self) -> TokenStream {
        let name = &self.name;
        let call = handle_call(&self.func);
        quote! {
            #name => {
                ::std::result::Result::Ok(::qi::value::IntoValue::into_value(#call.get()))
            }
        }
    }

    fn to_set_arm(&self) -> TokenStream {
        let name = &self.name;
        let call = handle_call(&self.func);
        let ty = payload_type(&self.func);
        quote! {
            #name => {
                let value: #ty = ::qi::value::Value::cast_into(raw_value)
                    .map_err(|err| ::qi::Error::from(::qi::BoxError::from(err)))?;
                #call.set(value);
                ::std::result::Result::Ok(::qi::value::IntoValue::into_value(()))
            }
        }
    }

    fn to_check_tokens(&self) -> TokenStream {
        let name = &self.name;
        let ty = payload_type(&self.func);
        let mismatch = meta_mismatch(name);
        quote! {
            {
                let ident = ::qi::object::MemberIdent::from(#name);
                let property = meta.property(&ident).ok_or_else(|| {
                    ::qi::Error::MethodNotFound(ident)
                })?;
                if property.signature != <#ty as ::qi::value::Reflect>::signature() {
                    return ::std::result::Result::Err(#mismatch);
                }
            }
        }
    }

    fn to_client_item(&self) -> TokenStream {
        let sig = &self.func.sig;
        let name = &self.name;
        quote! {
            #sig {
                ::qi::Property::remote(self.client.clone(), #name)
            }
        }
    }
}

fn client_ident(trait_name: &Ident) -> Ident {
    Ident::new(&format!("{}Client", trait_name), trait_name.span())
}

fn parameters_signature(func: &TraitItemFn) -> TokenStream {
    let tys = func.sig.inputs.iter().filter_map(|input| match input {
        FnArg::Typed(typed) => Some(&*typed.ty),
        _ => None,
    });
    quote! {
        {
            let fields: ::std::vec::Vec<::std::option::Option<::qi::value::Type>> = ::std::vec![
                #(<#tys as ::qi::value::Reflect>::ty(),)*
            ];
            ::qi::value::Signature::new(Some(::qi::value::Type::tuple_of(fields)))
        }
    }
}

fn meta_mismatch(name: &str) -> TokenStream {
    quote! {
        ::qi::Error::from(::std::io::Error::new(
            ::std::io::ErrorKind::InvalidData,
            ::std::format!("meta object member `{}` has a mismatched signature", #name),
        ))
    }
}

fn handle_call(func: &TraitItemFn) -> TokenStream {
    let ident = &func.sig.ident;
    if func.sig.asyncness.is_some() {
        quote!(self.#ident().await)
    } else {
        quote!(self.#ident())
    }
}

fn tagged_fn(item: &TraitItem, tag: &str) -> Result<Option<(TraitItemFn, String)>> {
    let func = match item {
        TraitItem::Fn(f) => f,
        _ => return Ok(None),
    };
    let Some(attr) = func
        .attrs
        .iter()
        .find(|attr| is_member_tag_attribute(attr, tag.trim_start_matches("qi::")))
    else {
        return Ok(None);
    };
    let name = parse_name_arg(attr, tag)?.unwrap_or_else(|| func.sig.ident.to_string());
    require_ref_self(func, tag)?;
    Ok(Some((func.clone(), name)))
}

fn require_ref_self(func: &TraitItemFn, tag: &str) -> Result<()> {
    match func.sig.receiver() {
        Some(recv) if recv.reference.is_some() && recv.mutability.is_none() => Ok(()),
        Some(recv) if recv.mutability.is_some() => Err(Error::new_spanned(
            recv,
            format!("`#[{tag}]` requires a `&self` receiver, not `&mut self`"),
        )),
        Some(recv) => Err(Error::new_spanned(
            recv,
            format!("`#[{tag}]` requires a `&self` receiver, not `self` by value"),
        )),
        None => Err(Error::new_spanned(
            &func.sig,
            format!("`#[{tag}]` requires a `&self` receiver"),
        )),
    }
}

fn parse_name_arg(attr: &Attribute, tag: &str) -> Result<Option<String>> {
    match &attr.meta {
        Meta::Path(_) => Ok(None),
        Meta::List(_) => attr.parse_args_with(|input: ParseStream| {
            let key: Ident = input.parse()?;
            if key != "name" {
                return Err(Error::new(
                    key.span(),
                    format!("unknown qi attribute argument `{key}`"),
                ));
            }
            input.parse::<syn::Token![=]>()?;
            match input.parse::<Lit>()? {
                Lit::Str(s) => Ok(Some(s.value())),
                other => Err(Error::new_spanned(other, "`name` must be a string literal")),
            }
        }),
        Meta::NameValue(_) => Err(Error::new_spanned(
            attr,
            format!("`#[{tag}]` takes `name = \"...\"` in parentheses"),
        )),
    }
}

fn reject_tagged_non_fn(item: &TraitItem) -> Result<()> {
    if matches!(item, TraitItem::Fn(_)) {
        return Ok(());
    }
    for attr in item_attrs(item) {
        let Some(leaf) = qi_leaf(attr) else {
            continue;
        };
        if leaf == "method" || leaf == "property" || leaf == "signal" {
            return Err(Error::new_spanned(
                attr,
                format!("`#[qi::{leaf}]` requires a function"),
            ));
        }
    }
    Ok(())
}

fn reject_unknown_qi_attrs(attrs: &[Attribute]) -> Result<()> {
    for attr in attrs {
        let Some(leaf) = qi_leaf(attr) else {
            continue;
        };
        if leaf != "method" && leaf != "property" && leaf != "signal" {
            return Err(Error::new_spanned(
                attr,
                format!("unknown qi attribute `{leaf}`"),
            ));
        }
    }
    Ok(())
}

fn strip_qi_attrs(item: &mut TraitItem) {
    let attrs = match item {
        TraitItem::Const(item) => &mut item.attrs,
        TraitItem::Fn(item) => &mut item.attrs,
        TraitItem::Type(item) => &mut item.attrs,
        TraitItem::Macro(item) => &mut item.attrs,
        _ => return,
    };
    attrs.retain(|attr| qi_leaf(attr).is_none());
}

fn qi_leaf(attr: &Attribute) -> Option<&Ident> {
    if attr.style != AttrStyle::Outer {
        return None;
    }
    let mut segments = attr.path().segments.iter();
    match (segments.next(), segments.next(), segments.next()) {
        (Some(first), Some(second), None) if first.ident == "qi" => Some(&second.ident),
        _ => None,
    }
}

fn item_attrs(item: &TraitItem) -> &[Attribute] {
    match item {
        TraitItem::Const(item) => &item.attrs,
        TraitItem::Fn(item) => &item.attrs,
        TraitItem::Type(item) => &item.attrs,
        TraitItem::Macro(item) => &item.attrs,
        _ => &[],
    }
}

fn attribute_outer_doc(attr: &Attribute) -> Option<LitStr> {
    if attr.style != AttrStyle::Outer {
        return None;
    }
    let MetaNameValue { path, value, .. } = attr.meta.require_name_value().ok()?;
    if !path.is_ident("doc") {
        return None;
    }
    let doc_str = match value {
        Expr::Lit(ExprLit {
            lit: Lit::Str(s), ..
        }) => s,
        _ => return None,
    };

    Some(doc_str.clone())
}

fn is_member_tag_attribute(attr: &Attribute, ty: &str) -> bool {
    qi_leaf(attr).is_some_and(|leaf| leaf == ty)
}

fn meta_object_ident(trait_name: &Ident) -> Ident {
    Ident::new(
        &format!(
            "{}_META_OBJECT",
            trait_name.to_string().to_case(Case::UpperSnake)
        ),
        trait_name.span(),
    )
}

fn member_docs(func: &TraitItemFn) -> Option<String> {
    let docs: Vec<String> = func
        .attrs
        .iter()
        .filter_map(attribute_outer_doc)
        .map(|doc| {
            let value = doc.value();
            value.strip_prefix(' ').unwrap_or(&value).to_owned()
        })
        .collect();
    if docs.is_empty() {
        None
    } else {
        Some(docs.join("\n"))
    }
}

fn method_parameters(func: &TraitItemFn) -> TokenStream {
    let parameters = func.sig.inputs.iter().filter_map(|input| match input {
        FnArg::Typed(typed) => Some(typed),
        _ => None,
    });
    let statements = parameters.enumerate().map(|(index, typed)| {
        let ty = &typed.ty;
        let set_name = match &*typed.pat {
            Pat::Ident(pat) => {
                let name = pat.ident.to_string();
                Some(quote! {
                    method.parameter(#index).set_name(#name);
                })
            }
            _ => None,
        };
        quote! {
            #set_name
            method.parameter(#index).set_type(<#ty as ::qi::value::Reflect>::ty());
        }
    });
    quote! {
        #(#statements)*
    }
}

fn method_return_type(func: &TraitItemFn) -> TokenStream {
    match &func.sig.output {
        ReturnType::Default => quote!(()),
        ReturnType::Type(_, ty) => {
            let ty = unwrap_result(ty);
            quote!(#ty)
        }
    }
}

fn payload_type(func: &TraitItemFn) -> TokenStream {
    match &func.sig.output {
        ReturnType::Default => quote!(()),
        ReturnType::Type(_, ty) => {
            let ty = unwrap_handle(ty);
            quote!(#ty)
        }
    }
}

/// `Property<T>` and `Signal<T>` describe `T` on the wire.
fn unwrap_handle(ty: &Type) -> &Type {
    let Type::Path(path) = ty else {
        return ty;
    };
    let Some(last) = path.path.segments.last() else {
        return ty;
    };
    if last.ident != "Property" && last.ident != "Signal" {
        return ty;
    }
    let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
        return ty;
    };
    match args.args.first() {
        Some(syn::GenericArgument::Type(inner)) => inner,
        _ => ty,
    }
}

fn unwrap_result(ty: &Type) -> &Type {
    result_ok_type(ty).unwrap_or(ty)
}

fn is_result(ty: &Type) -> bool {
    result_ok_type(ty).is_some()
}

fn result_ok_type(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let last = path.path.segments.last()?;
    if last.ident != "Result" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
        return None;
    };
    match args.args.first() {
        Some(syn::GenericArgument::Type(inner)) => Some(inner),
        _ => None,
    }
}

fn method_arg_idents(func: &TraitItemFn) -> Vec<Ident> {
    func.sig
        .inputs
        .iter()
        .filter_map(|input| match input {
            FnArg::Typed(typed) => Some(typed),
            _ => None,
        })
        .enumerate()
        .map(|(index, typed)| match &*typed.pat {
            Pat::Ident(pat) => pat.ident.clone(),
            _ => Ident::new(&format!("arg_{index}"), Span::call_site()),
        })
        .collect()
}
