use super::capabilities;
use futures::{future::BoxFuture, FutureExt};
use qi_messaging::capabilities::CapabilitiesMap;
use sealed::sealed;

#[sealed]
pub(crate) trait ServiceExt {
    fn call_authenticate(&mut self, authenticate: Authenticate)
        -> BoxFuture<'_, Result<(), Error>>;

    fn authenticate<P>(&mut self, parameters: &P) -> BoxFuture<'_, Result<(), Error>>
    where
        P: Parameters + Sync + ?Sized,
    {
        let mut capabilities = capabilities::local_map().clone();
        parameters.insert_into(&mut capabilities);
        self.call_authenticate(Authenticate { capabilities })
    }

    fn authenticate_anonymously(&mut self) -> BoxFuture<'_, Result<(), Error>> {
        self.authenticate(&())
    }
}

pub(crate) trait Parameters {
    fn insert_into(&self, capabilites: &mut CapabilitiesMap<'_>);
}

impl Parameters for () {
    fn insert_into(&self, _capabilites: &mut CapabilitiesMap<'_>) {
        // nothing
    }
}

#[sealed]
impl<S> ServiceExt for S
where
    S: tower::Service<Authenticate, Response = (), Error = Error> + Send,
    S::Future: Send,
{
    fn call_authenticate(
        &mut self,
        authenticate: Authenticate,
    ) -> BoxFuture<'_, Result<(), Error>> {
        use tower::ServiceExt;
        async { self.ready().await?.call(authenticate).await }.boxed()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Authenticate {
    pub(crate) capabilities: CapabilitiesMap<'static>,
}

macro_rules! declare_prefixed_key {
    (qi: $name:ident, $suffix:literal) => {
        const $name: &str = concat!("__qi_auth_", $suffix);
    };
    (user: $name:ident, $suffix:literal) => {
        const $name: &str = concat!("auth_", $suffix);
    };
}

declare_prefixed_key!(qi: ERROR_REASON_KEY, "err_reason");
declare_prefixed_key!(qi: STATE_KEY, "state");
declare_prefixed_key!(user: USER_KEY, "user");
declare_prefixed_key!(user: TOKEN_KEY, "token");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum State {
    Error = 1,
    Continue = 2,
    Done = 3,
}

// pub(crate) fn handle_request(_parameters: Parameters<'_>) -> Parameters<'_> {
//     // TODO: Implement a more restrictive authentication.
//     let mut capabilities = capabilities::local().clone();
//     capabilities.extend([(STATE_KEY, State::Done.to_u32().unwrap())]);
//     capabilities
// }

// pub(crate) fn check_response(result: &Parameters<'_>) -> Result<(), Failure> {
//     let dynamic_state = result
//         .get(STATE_KEY)
//         .ok_or(CheckResultError::NoStateValue)?
//         .clone();
//     let state = dynamic_state
//         .as_number()
//         .as_ref()
//         .and_then(Number::as_uint32)
//         .and_then(State::from_u32)
//         .ok_or_else(|| CheckResultError::StateUnknownValue(dynamic_state))?;
//     match state {
//         State::Continue => Err(CheckResultError::Continue),
//         // Technically the error case should not happen. If an authentication error
//         // occurred, the server should return a call error, not a call reply, and therefore
//         // we should not have a capability map to check.
//         State::Error => {
//             let err = result
//                 .get(ERROR_REASON_KEY)
//                 .and_then(Dynamic::as_string)
//                 .map(String::as_str)
//                 .unwrap_or_else(|| "unknown reason");
//             Err(CheckResultError::Refused(err.to_owned()))
//         }
//         State::Done => Ok(()),
//     }
// }

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("no authentication state value was found in the result")]
    NoStateValue,

    // #[error("the authentication state value has an unknown value \"{0}\"")]
    // UnknownStateValue(Dynamic),
    #[error("the authentication is incomplete and must be continued")]
    Continue,

    #[error("the authentication attempt was refused, reason is: {0}")]
    Refused(String),

    #[error(transparent)]
    Call(#[from] crate::Error),
}
