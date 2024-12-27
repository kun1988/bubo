use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use crossbeam::atomic::AtomicCell;
use once_cell::sync::OnceCell;
use thiserror::Error;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime, PrimitiveDateTime};

static CACHE_TIMESTAMP: AtomicCell<i64> = AtomicCell::new(0);

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
    static INSTANCE: OnceCell<i64> = OnceCell::new();

    INSTANCE.get_or_init(|| {
        start_timestamp_updater();
        1
    });
    CACHE_TIMESTAMP.load()
}

pub fn current_timestamp_sec() -> i64 {
    current_timestamp_ms() / 1000
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

fn start_timestamp_updater() {
    println!("start_timestamp_updater");
    thread::spawn(move || {
        loop {
            let now = SystemTime::now();
            let ts = now
                .duration_since(UNIX_EPOCH)
                .expect("Clock may have gone backward")
                .as_millis() as i64;
            CACHE_TIMESTAMP.store(ts);
            // 每 0.5ms 更新一次时间戳
            thread::sleep(std::time::Duration::from_micros(500));
        }
    });
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