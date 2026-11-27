//! Units of measurement, each with one name and one field-name suffix in
//! every API.
//!
//! A quantity whose unit is fixed carries it in its field's name, the unit's
//! [`Unit::suffix`]: `length_mm`, `weight_grams`. A quantity whose unit varies
//! with the value, such as a measurement one source gives in inches and
//! another in millimetres, is a [`Quantity`], keeping the unit it was given
//! in:
//!
//! ```
//! use reflectapi_conventions::units::{Quantity, Unit};
//!
//! #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
//! struct Part {
//!     /// Fixed unit: in the name.
//!     weight_grams: u32,
//!     /// Unit given with the value.
//!     width: Quantity,
//! }
//!
//! let part = Part {
//!     weight_grams: 120,
//!     width: Quantity { value: 12.5, unit: Unit::Millimeter },
//! };
//! assert_eq!(
//!     serde_json::to_string(&part).unwrap(),
//!     r#"{"weight_grams":120,"width":{"value":12.5,"unit":"millimeter"}}"#
//! );
//! ```
//!
//! On the wire a unit is its [name](Unit::name), `millimeter`. The server
//! also reads its [symbol](Unit::symbol), `mm`, but sends the name:
//!
//! ```
//! # use reflectapi_conventions::units::{Quantity, Unit};
//! let width: Quantity = serde_json::from_str(r#"{"value": 12.5, "unit": "mm"}"#).unwrap();
//! assert_eq!(width.unit, Unit::Millimeter);
//! assert_eq!(serde_json::to_string(&width.unit).unwrap(), r#""millimeter""#);
//! assert_eq!(Unit::Millimeter.symbol(), "mm");
//! assert_eq!(Unit::Millimeter.suffix(), Some("_mm"));
//!
//! // A unit the server doesn't know is refused.
//! assert!(serde_json::from_str::<Unit>(r#""furlong""#).is_err());
//! ```
//!
//! [`Unit`] is open in a response: units get added, and a client generated
//! before one keeps the value as an unknown unit instead of failing. Clients
//! get its names alone; the symbols and suffixes stay in this crate.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::OnceLock;

macro_rules! units {
    ($( $variant:ident: $name:literal, $symbol:literal, $suffix:expr $(, $alias:literal)*; )*) => {
        /// A unit of measurement, written as its name in `snake_case`, such
        /// as `millimeter`. A server also reads the unit's symbol, such as
        /// `mm`, but sends the name. Each case is documented with its symbol.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, reflectapi::Input, reflectapi::Output)]
        #[reflectapi(open)]
        #[non_exhaustive]
        pub enum Unit {
            $(
                #[doc = $symbol]
                #[serde(rename = $name)]
                $variant,
            )*
        }

        impl Unit {
            /// Every unit.
            pub const ALL: &'static [Unit] = &[$(Unit::$variant),*];

            /// The unit's name, its value on the wire, singular and in
            /// `snake_case`: `"millimeter"`.
            pub const fn name(self) -> &'static str {
                match self {
                    $(Unit::$variant => $name,)*
                }
            }

            /// The unit's symbol, for showing a quantity to a person: `"mm"`,
            /// `"°C"`, `"kWh"`. SI's where it has one.
            pub const fn symbol(self) -> &'static str {
                match self {
                    $(Unit::$variant => $symbol,)*
                }
            }

            /// The suffix of a field's name for a quantity in this unit:
            /// `Some("_mm")` for `length_mm`. `None` for a unit that is only
            /// a value of `unit`: multiples of a byte, since a size is a
            /// count of bytes, `_bytes`, and the week, month and year, whose
            /// length varies, so a period of them is a count with a unit.
            pub const fn suffix(self) -> Option<&'static str> {
                match self {
                    $(Unit::$variant => $suffix,)*
                }
            }

            /// Other spellings that reading accepts, besides the name and
            /// the symbol.
            const fn aliases(self) -> &'static [&'static str] {
                match self {
                    $(Unit::$variant => &[$($alias),*],)*
                }
            }
        }
    };
}

units! {
    // Count
    Quantity: "quantity", "pcs", Some("_count");
    Percent: "percent", "%", Some("_percent");
    PartPerMillion: "part_per_million", "ppm", Some("_ppm");

    // Length
    Micrometer: "micrometer", "µm", Some("_micrometers");
    Millimeter: "millimeter", "mm", Some("_mm");
    Centimeter: "centimeter", "cm", Some("_cm");
    Meter: "meter", "m", Some("_meters");
    Kilometer: "kilometer", "km", Some("_km");
    Inch: "inch", "in", Some("_inches");
    Foot: "foot", "ft", Some("_ft");
    Yard: "yard", "yd", Some("_yd");
    Mile: "mile", "mi", Some("_miles");

    // Area
    SquareMeter: "square_meter", "m²", Some("_m2");
    SquareFoot: "square_foot", "ft²", Some("_ft2");
    SquareInch: "square_inch", "in²", Some("_in2");
    SquareYard: "square_yard", "yd²", Some("_yd2");
    SquareMile: "square_mile", "mi²", Some("_mi2");

    // Volume
    Milliliter: "milliliter", "mL", Some("_ml"), "ml";
    Liter: "liter", "L", Some("_liters"), "l";
    FluidOunce: "fluid_ounce", "fl oz", Some("_fluid_ounces");
    Gallon: "gallon", "gal", Some("_gallons");
    CubicMeter: "cubic_meter", "m³", Some("_m3");
    CubicFoot: "cubic_foot", "ft³", Some("_ft3");
    CubicInch: "cubic_inch", "in³", Some("_in3");
    CubicYard: "cubic_yard", "yd³", Some("_yd3");

    // Mass
    Milligram: "milligram", "mg", Some("_mg");
    Gram: "gram", "g", Some("_grams");
    Kilogram: "kilogram", "kg", Some("_kg");
    Ounce: "ounce", "oz", Some("_oz");
    Pound: "pound", "lb", Some("_lb");

    // Density
    KilogramPerCubicMeter: "kilogram_per_cubic_meter", "kg/m³", Some("_kg_per_m3");
    GramPerCubicCentimeter: "gram_per_cubic_centimeter", "g/cm³", Some("_g_per_cm3");
    PoundPerCubicFoot: "pound_per_cubic_foot", "lb/ft³", Some("_lb_per_ft3");
    OuncePerCubicInch: "ounce_per_cubic_inch", "oz/in³", Some("_oz_per_in3");

    // Time
    Nanosecond: "nanosecond", "ns", Some("_ns");
    Microsecond: "microsecond", "µs", Some("_microseconds");
    Millisecond: "millisecond", "ms", Some("_ms");
    Second: "second", "s", Some("_seconds");
    Minute: "minute", "min", Some("_minutes");
    Hour: "hour", "h", Some("_hours");
    Day: "day", "d", Some("_days");
    Week: "week", "wk", None;
    Month: "month", "mo", None;
    Year: "year", "yr", None;

    // Angle
    Degree: "degree", "°", Some("_degrees");
    Radian: "radian", "rad", Some("_radians");

    // Temperature
    Celsius: "celsius", "°C", Some("_celsius");
    Fahrenheit: "fahrenheit", "°F", Some("_fahrenheit");
    Kelvin: "kelvin", "K", Some("_kelvin");

    // Pressure
    Pascal: "pascal", "Pa", Some("_pa");
    Kilopascal: "kilopascal", "kPa", Some("_kpa");
    Millibar: "millibar", "mbar", Some("_mbar");
    Bar: "bar", "bar", Some("_bar");
    Atmosphere: "atmosphere", "atm", Some("_atm");
    PoundPerSquareInch: "pound_per_square_inch", "psi", Some("_psi");

    // Speed
    MeterPerSecond: "meter_per_second", "m/s", Some("_m_per_s");
    KilometerPerHour: "kilometer_per_hour", "km/h", Some("_km_per_h");
    MilePerHour: "mile_per_hour", "mph", Some("_mph");
    Knot: "knot", "kn", Some("_knots");

    // Rotation
    RadianPerSecond: "radian_per_second", "rad/s", Some("_rad_per_s");
    DegreePerSecond: "degree_per_second", "°/s", Some("_degrees_per_s");
    RevolutionPerMinute: "revolution_per_minute", "rpm", Some("_rpm");
    DegreePerRevolution: "degree_per_revolution", "°/rev", Some("_degrees_per_rev");

    // Acceleration
    MeterPerSecondSquared: "meter_per_second_squared", "m/s²", Some("_m_per_s2");
    KilometerPerHourSquared: "kilometer_per_hour_squared", "km/h²", Some("_km_per_h2");
    MilePerHourSquared: "mile_per_hour_squared", "mi/h²", Some("_mph2");
    StandardGravity: "standard_gravity", "gₙ", Some("_standard_gravities");

    // Data
    Byte: "byte", "B", Some("_bytes");
    Kilobyte: "kilobyte", "kB", None;
    Megabyte: "megabyte", "MB", None;
    Gigabyte: "gigabyte", "GB", None;
    Terabyte: "terabyte", "TB", None;
    Petabyte: "petabyte", "PB", None;
    Kibibyte: "kibibyte", "KiB", None;
    Mebibyte: "mebibyte", "MiB", None;
    Gibibyte: "gibibyte", "GiB", None;
    Tebibyte: "tebibyte", "TiB", None;

    // Energy
    Joule: "joule", "J", Some("_joules");
    Kilojoule: "kilojoule", "kJ", Some("_kj");
    Megajoule: "megajoule", "MJ", Some("_megajoules");
    Gigajoule: "gigajoule", "GJ", Some("_gigajoules");
    Calorie: "calorie", "cal", Some("_calories");
    Kilocalorie: "kilocalorie", "kcal", Some("_kcal");
    WattHour: "watt_hour", "Wh", Some("_wh");
    KilowattHour: "kilowatt_hour", "kWh", Some("_kwh");

    // Force
    Newton: "newton", "N", Some("_newtons");
    Kilonewton: "kilonewton", "kN", Some("_kilonewtons");
    Meganewton: "meganewton", "MN", Some("_meganewtons");

    // Torque
    NewtonMeter: "newton_meter", "N·m", Some("_newton_meters");
    KilonewtonMeter: "kilonewton_meter", "kN·m", Some("_kilonewton_meters");
    PoundFoot: "pound_foot", "lb·ft", Some("_pound_feet");
    KilogramMeter: "kilogram_meter", "kgf·m", Some("_kilogram_meters");

    // Power
    Watt: "watt", "W", Some("_watts");
    Kilowatt: "kilowatt", "kW", Some("_kw");
    Megawatt: "megawatt", "MW", Some("_megawatts");
    Gigawatt: "gigawatt", "GW", Some("_gigawatts");
    WattPerSquareMeter: "watt_per_square_meter", "W/m²", Some("_watts_per_m2");

    // Current
    Microampere: "microampere", "µA", Some("_microamperes");
    Milliampere: "milliampere", "mA", Some("_milliamperes");
    Ampere: "ampere", "A", Some("_amperes");
    Kiloampere: "kiloampere", "kA", Some("_kiloamperes");

    // Charge
    Coulomb: "coulomb", "C", Some("_coulombs");
    MilliampereHour: "milliampere_hour", "mAh", Some("_mah");
    AmpereHour: "ampere_hour", "Ah", Some("_ah");

    // Voltage
    Microvolt: "microvolt", "µV", Some("_microvolts");
    Millivolt: "millivolt", "mV", Some("_millivolts");
    Volt: "volt", "V", Some("_volts");
    Kilovolt: "kilovolt", "kV", Some("_kilovolts");

    // Resistance
    Milliohm: "milliohm", "mΩ", Some("_milliohms");
    Ohm: "ohm", "Ω", Some("_ohms");
    Kiloohm: "kiloohm", "kΩ", Some("_kiloohms");
    Megaohm: "megaohm", "MΩ", Some("_megaohms");

    // Capacitance
    Microfarad: "microfarad", "µF", Some("_microfarads");
    Millifarad: "millifarad", "mF", Some("_millifarads");
    Farad: "farad", "F", Some("_farads");

    // Inductance
    Microhenry: "microhenry", "µH", Some("_microhenries");
    Millihenry: "millihenry", "mH", Some("_millihenries");
    Henry: "henry", "H", Some("_henries");

    // Conductance
    Microsiemens: "microsiemens", "µS", Some("_microsiemens");
    Millisiemens: "millisiemens", "mS", Some("_millisiemens");
    Siemens: "siemens", "S", Some("_siemens");

    // Frequency
    Hertz: "hertz", "Hz", Some("_hz");
    Kilohertz: "kilohertz", "kHz", Some("_khz");
    Megahertz: "megahertz", "MHz", Some("_megahertz");
    Gigahertz: "gigahertz", "GHz", Some("_gigahertz");

    // Magnetism
    Weber: "weber", "Wb", Some("_webers");
    Tesla: "tesla", "T", Some("_teslas");
    Millitesla: "millitesla", "mT", Some("_milliteslas");

    // Light
    Lux: "lux", "lx", Some("_lux");
    Lumen: "lumen", "lm", Some("_lumens");
    Candela: "candela", "cd", Some("_candelas");
    CandelaPerSquareMeter: "candela_per_square_meter", "cd/m²", Some("_cd_per_m2");

    // Radiation
    Becquerel: "becquerel", "Bq", Some("_becquerels");
    Gray: "gray", "Gy", Some("_grays");
    Sievert: "sievert", "Sv", Some("_sieverts");
    Millisievert: "millisievert", "mSv", Some("_millisieverts");
    Microsievert: "microsievert", "µSv", Some("_microsieverts");

    // Sound
    Decibel: "decibel", "dB", Some("_decibels");

    // Flow
    LiterPerSecond: "liter_per_second", "L/s", Some("_liters_per_s");
    LiterPerHour: "liter_per_hour", "L/h", Some("_liters_per_h");
    GallonPerMinute: "gallon_per_minute", "gal/min", Some("_gallons_per_minute");
    GallonPerHour: "gallon_per_hour", "gal/h", Some("_gallons_per_hour");
    CubicCentimeterPerMinute: "cubic_centimeter_per_minute", "cm³/min", Some("_cm3_per_min");

    // Consumption
    LiterPer100Kilometers: "liter_per_100_kilometers", "L/100 km", Some("_liters_per_100km");
    MilePerGallon: "mile_per_gallon", "mpg", Some("_miles_per_gallon");

    // Wire gauge
    AmericanWireGauge: "american_wire_gauge", "AWG", Some("_awg");
}

impl Unit {
    /// The unit a name, symbol or other accepted spelling stands for. The
    /// Greek letters mu and omega are read as the micro and ohm signs.
    fn from_text(text: &str) -> Option<Unit> {
        let text: Cow<'_, str> = if text.contains(['\u{3bc}', '\u{2126}']) {
            text.replace('\u{3bc}', "\u{b5}")
                .replace('\u{2126}', "\u{3a9}")
                .into()
        } else {
            text.into()
        };
        Unit::by_text().get(text.as_ref()).copied()
    }

    /// Every name, symbol and other accepted spelling, with its unit, built
    /// once. Names go in first, so a symbol can't shadow another unit's name.
    fn by_text() -> &'static HashMap<&'static str, Unit> {
        static BY_TEXT: OnceLock<HashMap<&'static str, Unit>> = OnceLock::new();
        BY_TEXT.get_or_init(|| {
            let mut by_text: HashMap<_, _> =
                Unit::ALL.iter().map(|&unit| (unit.name(), unit)).collect();
            for &unit in Unit::ALL {
                for &text in std::iter::once(&unit.symbol()).chain(unit.aliases()) {
                    by_text.entry(text).or_insert(unit);
                }
            }
            by_text
        })
    }
}

/// By hand, to read a symbol as well as a name. Names come first, so a symbol
/// can't shadow another unit's name.
impl<'de> serde::Deserialize<'de> for Unit {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = Unit;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a unit's name, such as `millimeter`, or its symbol, such as `mm`")
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Unit, E> {
                Unit::from_text(value)
                    .ok_or_else(|| E::invalid_value(serde::de::Unexpected::Str(value), &self))
            }

            fn visit_bytes<E: serde::de::Error>(self, value: &[u8]) -> Result<Unit, E> {
                match std::str::from_utf8(value) {
                    Ok(value) => self.visit_str(value),
                    Err(_) => Err(E::invalid_value(serde::de::Unexpected::Bytes(value), &self)),
                }
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

/// A value with its unit, for a quantity whose unit varies with the value:
/// `{"value": 12.5, "unit": "millimeter"}`. A quantity whose unit is fixed is
/// a number in a field named with the unit's suffix instead, such as
/// `length_mm`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    reflectapi::Input,
    reflectapi::Output,
)]
pub struct Quantity {
    pub value: f64,
    pub unit: Unit,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, HashSet};

    /// The units table of the guidelines, as it is written there.
    const TABLE: &str = "\
| Count | `quantity`: `_count`; `percent`: `_percent`; `part_per_million`: `_ppm` |
| Length | `micrometer`: `_micrometers`; `millimeter`: `_mm`; `centimeter`: `_cm`; `meter`: `_meters`; `kilometer`: `_km`; `inch`: `_inches`; `foot`: `_ft`; `yard`: `_yd`; `mile`: `_miles` |
| Area | `square_meter`: `_m2`; `square_foot`: `_ft2`; `square_inch`: `_in2`; `square_yard`: `_yd2`; `square_mile`: `_mi2` |
| Volume | `milliliter`: `_ml`; `liter`: `_liters`; `fluid_ounce`: `_fluid_ounces`; `gallon` (US): `_gallons`; `cubic_meter`: `_m3`; `cubic_foot`: `_ft3`; `cubic_inch`: `_in3`; `cubic_yard`: `_yd3` |
| Mass | `milligram`: `_mg`; `gram`: `_grams`; `kilogram`: `_kg`; `ounce` (mass): `_oz`; `pound` (mass): `_lb` |
| Density | `kilogram_per_cubic_meter`: `_kg_per_m3`; `gram_per_cubic_centimeter`: `_g_per_cm3`; `pound_per_cubic_foot`: `_lb_per_ft3`; `ounce_per_cubic_inch`: `_oz_per_in3` |
| Time | `nanosecond`: `_ns`; `microsecond`: `_microseconds`; `millisecond`: `_ms`; `second`: `_seconds`; `minute`: `_minutes`; `hour`: `_hours`; `day`: `_days`; `week`, `month`, `year`: a calendar period (`api-period-count-with-unit`) |
| Angle | `degree`: `_degrees`; `radian`: `_radians` |
| Temperature | `celsius`: `_celsius`; `fahrenheit`: `_fahrenheit`; `kelvin`: `_kelvin` |
| Pressure | `pascal`: `_pa`; `kilopascal`: `_kpa`; `millibar`: `_mbar`; `bar`: `_bar`; `atmosphere`: `_atm`; `pound_per_square_inch`: `_psi` |
| Speed | `meter_per_second`: `_m_per_s`; `kilometer_per_hour`: `_km_per_h`; `mile_per_hour`: `_mph`; `knot`: `_knots` |
| Rotation | `radian_per_second`: `_rad_per_s`; `degree_per_second`: `_degrees_per_s`; `revolution_per_minute`: `_rpm`; `degree_per_revolution`: `_degrees_per_rev` |
| Acceleration | `meter_per_second_squared`: `_m_per_s2`; `kilometer_per_hour_squared`: `_km_per_h2`; `mile_per_hour_squared`: `_mph2`; `standard_gravity`: `_standard_gravities` |
| Data | `byte`: `_bytes`; `kilobyte`, `megabyte`, `gigabyte`, `terabyte`, `petabyte` (powers of 1000) and `kibibyte`, `mebibyte`, `gibibyte`, `tebibyte` (powers of 1024): unit enum values only, since a field counts bytes (`api-size-in-bytes`) |
| Energy | `joule`: `_joules`; `kilojoule`: `_kj`; `megajoule`: `_megajoules`; `gigajoule`: `_gigajoules`; `calorie`: `_calories`; `kilocalorie`: `_kcal`; `watt_hour`: `_wh`; `kilowatt_hour`: `_kwh` |
| Force | `newton`: `_newtons`; `kilonewton`: `_kilonewtons`; `meganewton`: `_meganewtons` |
| Torque | `newton_meter`: `_newton_meters`; `kilonewton_meter`: `_kilonewton_meters`; `pound_foot`: `_pound_feet`; `kilogram_meter`: `_kilogram_meters` |
| Power | `watt`: `_watts`; `kilowatt`: `_kw`; `megawatt`: `_megawatts`; `gigawatt`: `_gigawatts`; `watt_per_square_meter`: `_watts_per_m2` |
| Current | `microampere`: `_microamperes`; `milliampere`: `_milliamperes`; `ampere`: `_amperes`; `kiloampere`: `_kiloamperes` |
| Charge | `coulomb`: `_coulombs`; `milliampere_hour`: `_mah`; `ampere_hour`: `_ah` |
| Voltage | `microvolt`: `_microvolts`; `millivolt`: `_millivolts`; `volt`: `_volts`; `kilovolt`: `_kilovolts` |
| Resistance | `milliohm`: `_milliohms`; `ohm`: `_ohms`; `kiloohm`: `_kiloohms`; `megaohm`: `_megaohms` |
| Capacitance | `microfarad`: `_microfarads`; `millifarad`: `_millifarads`; `farad`: `_farads` |
| Inductance | `microhenry`: `_microhenries`; `millihenry`: `_millihenries`; `henry`: `_henries` |
| Conductance | `microsiemens`: `_microsiemens`; `millisiemens`: `_millisiemens`; `siemens`: `_siemens` |
| Frequency | `hertz`: `_hz`; `kilohertz`: `_khz`; `megahertz`: `_megahertz`; `gigahertz`: `_gigahertz` |
| Magnetism | `weber`: `_webers`; `tesla`: `_teslas`; `millitesla`: `_milliteslas` |
| Light | `lux`: `_lux`; `lumen`: `_lumens`; `candela`: `_candelas`; `candela_per_square_meter`: `_cd_per_m2` |
| Radiation | `becquerel`: `_becquerels`; `gray`: `_grays`; `sievert`: `_sieverts`; `millisievert`: `_millisieverts`; `microsievert`: `_microsieverts` |
| Sound | `decibel`: `_decibels` |
| Flow | `liter_per_second`: `_liters_per_s`; `liter_per_hour`: `_liters_per_h`; `gallon_per_minute`: `_gallons_per_minute`; `gallon_per_hour`: `_gallons_per_hour`; `cubic_centimeter_per_minute`: `_cm3_per_min` |
| Consumption | `liter_per_100_kilometers`: `_liters_per_100km`; `mile_per_gallon`: `_miles_per_gallon` |
| Wire gauge | `american_wire_gauge`: `_awg` |";

    /// The table's units, by name, with their suffixes; `None` for the units
    /// it gives as enum values only.
    fn table() -> BTreeMap<&'static str, Option<&'static str>> {
        let mut units = BTreeMap::new();
        for row in TABLE.lines() {
            // Text in backticks, leaving out the rules a row refers to.
            let quoted = row
                .split('`')
                .skip(1)
                .step_by(2)
                .filter(|text| !text.starts_with("api-"));
            let mut name = None;
            for text in quoted {
                if text.starts_with('_') {
                    units.insert(name.take().unwrap(), Some(text));
                } else if let Some(previous) = name.replace(text) {
                    // A unit without a suffix, followed by another.
                    units.insert(previous, None);
                }
            }
            if let Some(previous) = name {
                units.insert(previous, None);
            }
        }
        units
    }

    #[test]
    fn every_unit_round_trips_by_name() {
        for unit in Unit::ALL {
            let json = serde_json::to_string(unit).unwrap();
            assert_eq!(json, format!("\"{}\"", unit.name()));
            assert_eq!(serde_json::from_str::<Unit>(&json).unwrap(), *unit);
        }
    }

    #[test]
    fn symbols_are_read_as_their_units() {
        for unit in Unit::ALL {
            let json = serde_json::to_string(unit.symbol()).unwrap();
            assert_eq!(
                serde_json::from_str::<Unit>(&json).unwrap(),
                *unit,
                "{json}"
            );
        }
        let read = |text: &str| serde_json::from_value::<Unit>(text.into()).unwrap();
        assert_eq!(read("mm"), Unit::Millimeter);
        assert_eq!(read("ml"), Unit::Milliliter);
        assert_eq!(read("l"), Unit::Liter);
        assert_eq!(read("\u{3bc}m"), Unit::Micrometer);
        assert_eq!(read("k\u{2126}"), Unit::Kiloohm);
        assert_eq!(read("°C"), Unit::Celsius);
        assert_eq!(read("kWh"), Unit::KilowattHour);
        assert_eq!(read("N·m"), Unit::NewtonMeter);
        // Case keeps symbols apart.
        assert_eq!(read("ms"), Unit::Millisecond);
        assert_eq!(read("mS"), Unit::Millisiemens);
        assert_eq!(read("MW"), Unit::Megawatt);
        assert!(serde_json::from_str::<Unit>(r#""MM""#).is_err());
        assert!(serde_json::from_str::<Unit>(r#""Millimeter""#).is_err());
    }

    #[test]
    fn suffixes_are_the_tables() {
        let units: BTreeMap<_, _> = Unit::ALL
            .iter()
            .map(|unit| (unit.name(), unit.suffix()))
            .collect();
        assert_eq!(units, table());
        assert_eq!(units.len(), Unit::ALL.len());
    }

    #[test]
    fn names_suffixes_and_spellings_are_unique() {
        let mut names = HashSet::new();
        let mut suffixes = HashSet::new();
        for unit in Unit::ALL {
            assert!(names.insert(unit.name()), "{}", unit.name());
            if let Some(suffix) = unit.suffix() {
                assert!(suffixes.insert(suffix), "{suffix}");
            }
        }
        // A symbol or other spelling stands for one unit, and isn't another
        // unit's name.
        let mut spellings = HashSet::new();
        for unit in Unit::ALL {
            for spelling in std::iter::once(unit.symbol()).chain(unit.aliases().iter().copied()) {
                assert!(spellings.insert(spelling), "{spelling}");
                assert!(
                    !names.contains(spelling) || spelling == unit.name(),
                    "{spelling}"
                );
            }
        }
    }

    #[test]
    fn a_quantity_is_its_value_and_unit() {
        let quantity = Quantity {
            value: 12.5,
            unit: Unit::Millimeter,
        };
        let json = r#"{"value":12.5,"unit":"millimeter"}"#;
        assert_eq!(serde_json::to_string(&quantity).unwrap(), json);
        assert_eq!(serde_json::from_str::<Quantity>(json).unwrap(), quantity);
    }
}
