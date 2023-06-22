use super::capabilities::{self, CapabilitiesMap};
use num_traits::{FromPrimitive, ToPrimitive};
use qi_types::{Dynamic, Number};

macro_rules! declare_prefixed_key {
    ($name:ident, $suffix:literal) => {
        const $name: &str = concat!("__qi_auth_", $suffix);
    };
    ($name:ident) => {
        declare_prefixed_key!($name, "");
    };
}

// declare_prefixed_key!(PREFIX);
declare_prefixed_key!(ERROR_REASON_KEY, "err_reason");
declare_prefixed_key!(STATE_KEY, "state");
// const USER_AUTH_PREFIX: &str = "auth_";

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::Display,
    num_derive::FromPrimitive,
    num_derive::ToPrimitive,
)]
#[repr(u32)]
enum State {
    #[display(fmt = "error")]
    Error = 1,
    #[display(fmt = "continue")]
    Continue = 2,
    #[display(fmt = "done")]
    Done = 3,
}

pub(super) fn authenticate(_parameters: &CapabilitiesMap) -> CapabilitiesMap {
    // TODO: Implement a more restrictive authentication.
    let mut capabilities = capabilities::local().clone();
    capabilities.extend([(STATE_KEY, State::Done.to_u32().unwrap())]);
    capabilities
}

pub(super) fn verify_result(result: &CapabilitiesMap) -> Result<(), VerifyResultError> {
    let dynamic_state = result
        .get(STATE_KEY)
        .ok_or(VerifyResultError::NoStateValue)?
        .clone();
    let state = dynamic_state
        .as_number()
        .as_ref()
        .and_then(Number::as_uint32)
        .and_then(State::from_u32)
        .ok_or_else(|| VerifyResultError::StateUnknownValue(dynamic_state))?;
    match state {
        State::Continue => Err(VerifyResultError::Continue),
        // Technically the error case should not happen. If an authentication error
        // occurred, the server should return a call error, not a call reply, and therefore
        // we should not have a capability map to check.
        State::Error => {
            let reason = result
                .get(ERROR_REASON_KEY)
                .and_then(Dynamic::as_string)
                .map(String::as_str)
                .unwrap_or_else(|| "unknown reason");
            Err(VerifyResultError::Error(reason.to_owned()))
        }
        State::Done => Ok(()),
    }
}

#[derive(Debug, thiserror::Error)]
pub(in crate::session) enum VerifyResultError {
    #[error("no authentication state value was found in the result")]
    NoStateValue,

    #[error("the authentication state value has an unknown value \"{0}\"")]
    StateUnknownValue(Dynamic),

    #[error("the authentication is incomplete and must be continued")]
    Continue,

    #[error("the authentication ended with an error: {0}")]
    Error(String),
}
