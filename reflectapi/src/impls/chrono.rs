fn reflectapi_date_time(schema: &mut crate::Typespace) -> String {
    let type_name = "chrono::DateTime";
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "DateTime at a given timezone (RFC3339 format)".into(),
            vec!["Tz".into()],
            Some("std::string::String".into()),
        );
        schema.insert_type(type_def.into());
    }
    type_name.into()
}
impl crate::Input for chrono::DateTime<chrono::Utc> {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(reflectapi_date_time(schema), vec!["chrono::Utc".into()])
    }
}
impl crate::Output for chrono::DateTime<chrono::Utc> {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(reflectapi_date_time(schema), vec!["chrono::Utc".into()])
    }
}
impl crate::Input for chrono::DateTime<chrono::Local> {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(reflectapi_date_time(schema), vec!["chrono::Local".into()])
    }
}
impl crate::Output for chrono::DateTime<chrono::Local> {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(reflectapi_date_time(schema), vec!["chrono::Local".into()])
    }
}
impl crate::Input for chrono::DateTime<chrono::FixedOffset> {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_date_time(schema),
            vec!["chrono::FixedOffset".into()],
        )
    }
}
impl crate::Output for chrono::DateTime<chrono::FixedOffset> {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_date_time(schema),
            vec!["chrono::FixedOffset".into()],
        )
    }
}

fn reflectapi_naive_datetime(schema: &mut crate::Typespace) -> String {
    let type_name = "chrono::NaiveDateTime";
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "Date time without timezone (%Y-%m-%dT%H:%M:%S%.f)".into(),
            vec![],
            Some("std::string::String".into()),
        );
        schema.insert_type(type_def.into());
    }
    type_name.into()
}

impl crate::Input for chrono::NaiveDateTime {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(reflectapi_naive_datetime(schema), vec![])
    }
}
impl crate::Output for chrono::NaiveDateTime {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(reflectapi_naive_datetime(schema), vec![])
    }
}

fn reflectapi_naive_date(schema: &mut crate::Typespace) -> String {
    let type_name = "chrono::NaiveDate";
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "Date without timezone (%Y-%m-%d)".into(),
            vec![],
            Some("std::string::String".into()),
        );
        schema.insert_type(type_def.into());
    }
    type_name.into()
}

impl crate::Input for chrono::NaiveDate {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(reflectapi_naive_date(schema), vec![])
    }
}

impl crate::Output for chrono::NaiveDate {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(reflectapi_naive_date(schema), vec![])
    }
}

fn reflectapi_naive_time(schema: &mut crate::Typespace) -> String {
    let type_name = "chrono::NaiveTime";
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "Time without timezone (%H:%M:%S%.f)".into(),
            vec![],
            Some("std::string::String".into()),
        );
        schema.insert_type(type_def.into());
    }
    type_name.into()
}

impl crate::Input for chrono::NaiveTime {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(reflectapi_naive_time(schema), vec![])
    }
}
impl crate::Output for chrono::NaiveTime {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(reflectapi_naive_time(schema), vec![])
    }
}

// chrono does not provide serde implementation for chrono::Duration
// and other chrono types not described above
