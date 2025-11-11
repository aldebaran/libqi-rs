use proc_macro2::TokenStream;

pub(super) struct Object {
    methods: Vec<Method>,
}

impl Object {
    pub(super) fn generate(self) -> TokenStream {
        todo!()
    }
}

impl syn::parse::Parse for Object {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _item = syn::ItemTrait::parse(input)?;
        todo!()
    }
}

struct Method {}

impl syn::parse::Parse for Method {
    fn parse(_input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {})
    }
}
