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

impl Input for i8 {
    fn reflect_input() -> crate::Schema {
        let mut schema = crate::Schema::new();
        schema.reserve_type("i8".to_string());
        schema
    }
    fn reflect_input_name() -> String {
        "i8".to_string()
    }
    fn reflect_input_type(schema: &mut Schema) -> () {
        schema.insert_type(crate::Type {
            name: "i8".to_string(),
            fields: Vec::new(),
            _debug: String::new(),
        });
    }
}

impl Output for i8 {
    fn reflect_output() -> crate::Schema {
        let mut schema = crate::Schema::new();
        schema.reserve_type("i8".to_string());
        schema
    }
    fn reflect_output_name() -> String {
        "i8".to_string()
    }
    fn reflect_output_type(schema: &mut Schema) -> () {
        schema.insert_type(crate::Type {
            name: "i8".to_string(),
            fields: Vec::new(),
            _debug: String::new(),
        });
    }
}
