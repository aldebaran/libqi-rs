use std::{convert::Infallible, str::FromStr};

#[derive(Debug, clap::Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = "tcp://[::1]:9559")]
    pub address: qi::Address,

    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[arg(short, long)]
    pub user_and_token: Option<UserAndToken>,
}

#[derive(Debug, Clone)]
pub struct UserAndToken {
    pub user: String,
    pub token: String,
}

impl FromStr for UserAndToken {
    type Err = UserAndTokenError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(':');

        let user = parts
            .next()
            .ok_or(UserAndTokenError::MissingColon)?
            .to_owned();
        let token = parts
            .next()
            .ok_or(UserAndTokenError::MissingColon)?
            .to_owned();
        if parts.next().is_some() {
            return Err(UserAndTokenError::TooManyColons);
        }
        Ok(Self { user, token })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UserAndTokenError {
    #[error("missing colon")]
    MissingColon,
    #[error("too many colons")]
    TooManyColons,
}
