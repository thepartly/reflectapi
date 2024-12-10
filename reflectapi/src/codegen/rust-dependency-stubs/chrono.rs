#[derive(Debug)]
pub struct Local;

#[derive(Debug)]
pub struct FixedOffset;

#[derive(Debug)]
pub struct DateTime<Tz> {
    _tz: Tz,
}

#[derive(Debug)]
pub struct Utc;

#[derive(Debug)]
pub struct NaiveDateTime;

#[derive(Debug)]
pub struct NaiveDate;

#[derive(Debug)]
pub struct NaiveTime;
