#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Never {}

impl std::fmt::Display for Never {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {}
    }
}

impl From<std::convert::Infallible> for Never {
    fn from(v: std::convert::Infallible) -> Self {
        match v {}
    }
}

impl From<Never> for std::convert::Infallible {
    fn from(v: Never) -> Self {
        match v {}
    }
}

impl std::error::Error for Never {}
