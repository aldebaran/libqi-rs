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
    fn meta(&self) -> qi::object::MetaObject {
        todo!()
    }

    async fn meta_call(
        &self,
        address: qi::object::MemberAddress,
        args: qi::Value<'_>,
    ) -> Result<qi::Value<'static>, qi::Error> {
        todo!()
    }
}
