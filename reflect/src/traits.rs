pub trait Input {
    fn reflect_input() -> crate::Schema;
}

pub trait Output {
    fn reflect_output() -> crate::Schema;
}
