use qi_messaging::CapabilitiesMap;
use qi_value::{Dynamic, Value};
use sealed::sealed;
use std::collections::HashMap;

pub trait Authenticator {
    fn verify(&self, parameters: HashMap<String, Value<'_>>) -> Result<(), Error>;
}

pub struct PermissiveAuthenticator;

impl Authenticator for PermissiveAuthenticator {
    fn verify(&self, _parameters: HashMap<String, Value<'_>>) -> Result<(), Error> {
        Ok(())
    }
}

pub struct UserTokenAuthenticator {
    user: String,
    token: String,
}

impl UserTokenAuthenticator {
    pub fn new(user: String, token: String) -> Self {
        Self { user, token }
    }
}

impl Authenticator for UserTokenAuthenticator {
    fn verify(&self, mut parameters: HashMap<String, Value<'_>>) -> Result<(), Error> {
        let user: &str = parameters
            .remove(USER_KEY)
            .ok_or_else(|| Error::UserValue("missing".to_owned()))?
            .cast()
            .map_err(|err| Error::UserValue(err.to_string()))?;
        let token: &str = parameters
            .remove(TOKEN_KEY)
            .ok_or_else(|| Error::TokenValue("missing".to_owned()))?
            .cast()
            .map_err(|err| Error::TokenValue(err.to_string()))?;
        (user == self.user && token == self.token)
            .then_some(())
            .ok_or_else(|| Error::Refused("invalid user/token credentials".to_owned()))
    }
}

#[sealed]
pub(crate) trait CapabilitiesMapExt {
    fn insert_authentication_state_done(&mut self);
    fn to_authentication_result(&self) -> Result<(), Error>;
}

#[sealed]
impl CapabilitiesMapExt for CapabilitiesMap {
    fn insert_authentication_state_done(&mut self) {
        self.insert(STATE_KEY.to_owned(), Dynamic(Value::UInt32(STATE_DONE)));
    }

    fn to_authentication_result(&self) -> Result<(), Error> {
        let Dynamic(state) = self
            .get(STATE_KEY)
            .ok_or_else(|| Error::StateValue("missing".to_owned()))?
            .clone();
        match state {
            Value::UInt32(STATE_DONE) => Ok(()),
            _ => Err(Error::StateValue(format!(
                "expected a \"Done\" state value of \"{}u32\", found \"{}\" instead",
                STATE_DONE, state
            ))),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("user value error: {0}")]
    UserValue(String),

    #[error("token value error: {0}")]
    TokenValue(String),

    #[error("state value error: {0}")]
    StateValue(String),

    #[error("the authentication attempt must be continued, but authentication continuation is unsupported")]
    UnsupportedContinue,

    #[error("the authentication attempt was refused, reason is: {0}")]
    Refused(String),
}

macro_rules! declare_prefixed_key {
    (qi: $name:ident, $suffix:literal) => {
        const $name: &str = concat!("__qi_auth_", $suffix);
    };
    (user: $name:ident, $suffix:literal) => {
        const $name: &str = concat!("auth_", $suffix);
    };
}

// declare_prefixed_key!(qi: ERROR_REASON_KEY, "err_reason");
declare_prefixed_key!(qi: STATE_KEY, "state");
declare_prefixed_key!(user: USER_KEY, "user");
declare_prefixed_key!(user: TOKEN_KEY, "token");

const STATE_DONE: u32 = 3;
