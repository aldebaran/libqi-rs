use async_trait::async_trait;

#[derive(Default, Debug)]
pub(crate) struct Player;

impl Player {
    pub(crate) fn new() -> Self {
        Self
    }
}

#[async_trait]
impl qi::Object for Player {
    fn meta(&self) -> &qi::object::MetaObject {
        todo!()
    }

    async fn meta_call(
        &self,
        ident: qi::object::MemberIdent,
        args: qi::Value<'_>,
    ) -> qi::Result<qi::Value<'static>> {
        todo!()
    }

    async fn meta_post(
        &self,
        ident: qi::object::MemberIdent,
        value: qi::Value<'_>,
    ) -> qi::Result<()> {
        todo!()
    }

    async fn meta_event(
        &self,
        ident: qi::object::MemberIdent,
        value: qi::Value<'_>,
    ) -> qi::Result<()> {
        todo!()
    }
}
