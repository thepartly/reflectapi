pub struct Bytes {}

impl std::ops::Deref for Bytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unimplemented!()
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(_v: Vec<u8>) -> Self {
        unimplemented!()
    }
}

pub struct BytesMut {}

impl BytesMut {
    pub fn with_capacity(_capacity: usize) -> Self {
        unimplemented!()
    }

    pub fn extend_from_slice(&mut self, _extend: &[u8]) {
        unimplemented!()
    }

    pub fn freeze(self) -> Bytes {
        unimplemented!()
    }
}
