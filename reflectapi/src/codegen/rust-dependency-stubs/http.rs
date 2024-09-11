#[derive(Debug)]
pub struct StatusCode {}

impl core::fmt::Display for StatusCode {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        unimplemented!()
    }
}


impl StatusCode {
    pub fn is_client_error(&self) -> bool {
        unimplemented!()
    }

    pub fn is_success(&self) -> bool {
        unimplemented!()
    }
}
