use reflect_schema::Schema;

pub trait Input {
    fn reflect_input() -> crate::Schema;
    fn reflect_input_name() -> String;
    fn reflect_input_type(schema: &mut Schema) -> ();
}

pub trait Output {
    fn reflect_output() -> crate::Schema;
    fn reflect_output_name() -> String;
    fn reflect_output_type(schema: &mut Schema) -> ();
}
