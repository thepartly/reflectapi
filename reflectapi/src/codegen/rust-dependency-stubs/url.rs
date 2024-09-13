#[derive(Debug, Clone)]
pub struct Url {}

impl Url {
    pub fn join(&self, path: &str) -> Result<Url, ParseError> {
        todo!()
    }

    pub fn cannot_be_a_base(&self) -> bool {
        todo!()
    }
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[derive(Debug)]
pub enum ParseError {
    RelativeUrlWithCannotBeABaseBase,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::str::FromStr for Url {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}
