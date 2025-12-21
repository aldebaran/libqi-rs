use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    AttrStyle, Attribute, Expr, ExprLit, Ident, ItemTrait, Lit, LitStr, Meta, MetaNameValue,
    Result, TraitItem, TraitItemFn,
};

#[derive(Debug)]
pub(super) struct Object {
    trait_item: ItemTrait,
    name: String,
    methods: Vec<Method>,
    signals: Vec<Signal>,
    properties: Vec<Property>,
    description: Vec<LitStr>,
}

impl ToTokens for Object {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.trait_item.to_tokens(tokens)
    }
}

impl Parse for Object {
    fn parse(input: ParseStream) -> Result<Self> {
        let trait_item = ItemTrait::parse(input)?;

        let description = trait_item
            .attrs
            .iter()
            .filter_map(attribute_outer_doc)
            .collect();

        let items_len = trait_item.items.len();
        let mut methods = Vec::with_capacity(items_len);
        let mut signals = Vec::with_capacity(items_len);
        let mut properties = Vec::with_capacity(items_len);

        for item in &trait_item.items {
            if let Some(method) = Method::from_item(item) {
                methods.push(method)
            } else if let Some(signal) = Signal::from_item(item) {
                signals.push(signal)
            } else if let Some(property) = Property::from_item(item) {
                properties.push(property)
            }
        }

        Ok(Self {
            name: trait_item.ident.to_string(),
            trait_item,
            methods,
            signals,
            properties,
            description,
        })
    }
}

#[derive(Debug)]
struct Method {
    func: TraitItemFn,
}

impl Method {
    fn from_item(item: &TraitItem) -> Option<Self> {
        let func = match item {
            TraitItem::Fn(f) => f,
            _ => return None,
        };

        if !func
            .attrs
            .iter()
            .any(|attr| is_member_tag_attribute(attr, "method"))
        {
            return None;
        }

        let name = func.sig.ident.clone();

        Some(Self { func: func.clone() })
    }
}

#[derive(Debug)]
struct Signal {
    func: TraitItemFn,
}

impl Signal {
    fn from_item(item: &TraitItem) -> Option<Self> {
        let func = match item {
            TraitItem::Fn(f) => f,
            _ => return None,
        };

        if !func
            .attrs
            .iter()
            .any(|attr| is_member_tag_attribute(attr, "signal"))
        {
            return None;
        }

        Some(Self { func: func.clone() })
    }
}

#[derive(Debug)]
struct Property {
    func: TraitItemFn,
}

impl Property {
    fn from_item(item: &TraitItem) -> Option<Self> {
        let func = match item {
            TraitItem::Fn(f) => f,
            _ => return None,
        };

        if !func
            .attrs
            .iter()
            .any(|attr| is_member_tag_attribute(attr, "property"))
        {
            return None;
        }

        Some(Self { func: func.clone() })
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
    if attr.style != AttrStyle::Outer {
        return false;
    }

    let path = match &attr.meta {
        Meta::Path(path) => path,
        _ => return false,
    };

    if path.segments.len() != 2 {
        return false;
    }

    path.segments.iter().map(|seg| &seg.ident).eq(["qi", ty])
}
