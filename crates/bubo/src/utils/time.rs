use thiserror::Error;
use time::convert::{Millisecond, Second};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime, PrimitiveDateTime};

pub fn unix_epoch() -> OffsetDateTime {
    OffsetDateTime::UNIX_EPOCH
}

pub fn now_utc() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

pub fn now_utc_primitive() -> PrimitiveDateTime {
    let now = OffsetDateTime::now_utc();
    PrimitiveDateTime::new(now.date(), now.time())
}

pub fn current_timestamp_ms() -> i64 {
    let now = OffsetDateTime::now_utc();
    now.unix_timestamp() * Millisecond::per(Second) as i64 + now.millisecond() as i64
}

pub fn current_timestamp_sec() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

pub fn format_time(time: OffsetDateTime) -> String {
    time.format(&Rfc3339).unwrap() // TODO: need to check if safe.
}

pub fn now_utc_plus_sec_str(sec: f64) -> String {
    let new_time = now_utc() + Duration::seconds_f64(sec);
    format_time(new_time)
}

pub fn parse_utc(moment: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(moment, &Rfc3339)
        .map_err(|_| Error::DateFailParse(moment.to_string()))
}

// region:    --- Error

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("日期解析失败:{0}")]
    DateFailParse(String),
}

// region:    --- Error Boilerplate
// endregion: --- Error Boilerplate

// endregion: --- Error