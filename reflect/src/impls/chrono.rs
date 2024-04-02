fn reflect_date_time(schema: &mut crate::Typespace, timezone: &str) -> String {
    let type_name = "chrono::DateTime";
    if schema.reserve_type(&type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            format!(
                "DateTime at {} timezone",
                timezone.split("::").last().unwrap()
            ),
            vec![timezone.into()],
            Some("std::string::String".into()),
        );
        schema.insert_type(type_def.into());
    }
    type_name.into()
}
impl crate::Input for chrono::DateTime<chrono::Utc> {
    fn reflect_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        reflect_date_time(schema, "chrono::Utc").into()
    }
}
impl crate::Output for chrono::DateTime<chrono::Utc> {
    fn reflect_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        reflect_date_time(schema, "chrono::Utc").into()
    }
}
impl crate::Input for chrono::DateTime<chrono::Local> {
    fn reflect_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        reflect_date_time(schema, "chrono::Local").into()
    }
}
impl crate::Output for chrono::DateTime<chrono::Local> {
    fn reflect_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        reflect_date_time(schema, "chrono::Local").into()
    }
}
impl crate::Input for chrono::DateTime<chrono::FixedOffset> {
    fn reflect_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        reflect_date_time(schema, "chrono::FixedOffset").into()
    }
}
impl crate::Output for chrono::DateTime<chrono::FixedOffset> {
    fn reflect_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        reflect_date_time(schema, "chrono::FixedOffset").into()
    }
}

impl crate::Input for chrono::NaiveDateTime {
    fn reflect_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::reflect_type_simple(
            schema,
            "chrono::NaiveDateTime",
            "Date time without timezone",
            Some("std::string::String".into()),
        )
    }
}
impl crate::Output for chrono::NaiveDateTime {
    fn reflect_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::reflect_type_simple(
            schema,
            "chrono::NaiveDateTime",
            "Date time without timezone",
            Some("std::string::String".into()),
        )
    }
}

impl crate::Input for chrono::NaiveDate {
    fn reflect_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::reflect_type_simple(
            schema,
            "chrono::NaiveDate",
            "Date without timezone",
            Some("std::string::String".into()),
        )
    }
}
impl crate::Output for chrono::NaiveDate {
    fn reflect_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::reflect_type_simple(
            schema,
            "chrono::NaiveDate",
            "Date without timezone",
            Some("std::string::String".into()),
        )
    }
}

impl crate::Input for chrono::NaiveTime {
    fn reflect_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::reflect_type_simple(
            schema,
            "chrono::NaiveTime",
            "Time without timezone",
            Some("std::string::String".into()),
        )
    }
}
impl crate::Output for chrono::NaiveTime {
    fn reflect_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::reflect_type_simple(
            schema,
            "chrono::NaiveTime",
            "Time without timezone",
            Some("std::string::String".into()),
        )
    }
}

// chrono does not provide serde implementation for chrono::Duration
// and other chrono types not described above
