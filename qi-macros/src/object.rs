use proc_macro2::TokenStream;

#[derive(Debug)]
pub(crate) struct Object;

impl syn::parse::Parse for Object {
    fn parse(_input: syn::parse::ParseStream) -> syn::Result<Self> {
        todo!()
    }
}

impl Object {
    pub(crate) fn derive(self) -> TokenStream {
        todo!()
    }
}
