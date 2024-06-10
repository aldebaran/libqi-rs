use qi_messaging::CapabilitiesMap;
use qi_value::{Dynamic, Value};
use std::collections::HashMap;

pub type Parameters<'a> = HashMap<String, Value<'a>>;

pub trait Authenticator {
    fn verify(&self, parameters: Parameters) -> Result<(), Error>;
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PermissiveAuthenticator;

impl Authenticator for PermissiveAuthenticator {
    fn verify(&self, _parameters: Parameters) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct UserTokenAuthenticator {
    user: String,
    token: String,
}

impl Authenticator for UserTokenAuthenticator {
    fn verify(&self, mut parameters: Parameters) -> Result<(), Error> {
        let user: &str = parameters
            .remove(USER_KEY)
            .ok_or_else(|| Error::UserValue("missing".to_owned()))?
            .cast_into()
            .map_err(|err| Error::UserValue(err.to_string()))?;
        let token: &str = parameters
            .remove(TOKEN_KEY)
            .ok_or_else(|| Error::TokenValue("missing".to_owned()))?
            .cast_into()
            .map_err(|err| Error::TokenValue(err.to_string()))?;
        (user == self.user && token == self.token)
            .then_some(())
            .ok_or_else(|| Error::Refused("invalid user/token credentials".to_owned()))
    }
}

pub(super) fn state_done_map(mut capabilities: CapabilitiesMap) -> CapabilitiesMap {
    capabilities.insert(STATE_KEY.to_owned(), Dynamic(Value::UInt32(STATE_DONE)));
    capabilities
}

pub(super) fn extract_state_result(capabilities: &mut CapabilitiesMap) -> Result<(), Error> {
    let Dynamic(state) = capabilities
        .remove(STATE_KEY)
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
