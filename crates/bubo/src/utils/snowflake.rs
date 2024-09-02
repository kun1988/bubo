
use std::sync::{Arc, Mutex};

use once_cell::sync::OnceCell;
use tracing::error;

use super::time::current_timestamp_ms;

/// 开始时间截(2019-11-27)
const EPOCH: i64 = 1574784000000;
/// 序列所占位数(12)
const SEQUENCE_BITS: i64 = 12;
/// 机器id所占位数(10)
const WORKER_ID_BITS: i64 = 10;
/// 机器id掩码
const WORKER_ID_MASK: i64 = -1 ^ (-1 << WORKER_ID_BITS);
/// 机器id左移位数
const WORKER_ID_LEFT_SHIFT: i64 = SEQUENCE_BITS;
/// 时间戳左移位数
const TIMESTAMP_LEFT_SHIFT: i64 = WORKER_ID_LEFT_SHIFT + WORKER_ID_BITS;
/// 生成序列的掩码，这里为4095 (0b111111111111=0xfff=4095)
const SEQUENCE_MASK: i64 = -1 ^ (-1 << SEQUENCE_BITS);
/// 生成序列的最大值，这里为4095 (0b111111111111=0xfff=4095)
// const MAX_SEQUENCE: i64 = -1 ^ (-1 << SEQUENCE_BITS);
/// 最大时间戳， 41位
const MAX_TIME: i64 = (1 << 41) -1;

pub struct Snowflake {
    inner: Mutex<TimestampSequence>,
    worker_id: i64,
}

#[derive(Copy, Clone)]
struct TimestampSequence {
    timestamp: i64,
    sequence: i64,
    begin_id: i64,
}

impl TimestampSequence {
    fn new() -> Self {
        Self {
            timestamp: EPOCH,
            sequence: 0,
            begin_id: 0,
        }
    }
}

impl Snowflake {
    pub fn new(worker_id: u16) -> Self {
        Snowflake {
            worker_id: worker_id as i64,
            inner: Mutex::new(TimestampSequence::new()),
        }
    }

    pub fn next_id(&self) -> i64 {
        let mut timestamp_sequence = self.inner.lock().unwrap();
        // 当前时间
        let mut timestamp = get_timestamp();
        // 如果当前时间小于上一次ID生成的时间戳，说明系统时钟回退过这个时候应当抛出异常
        if timestamp < timestamp_sequence.timestamp {
            error!("时钟回退{},{}",timestamp_sequence.timestamp, timestamp);
            timestamp = timestamp_sequence.timestamp;
        }

        // 如果是同一时间生成的，则进行毫秒内序列
        if timestamp_sequence.timestamp == timestamp {
            timestamp_sequence.sequence = (timestamp_sequence.sequence + 1) & SEQUENCE_MASK;
            //毫秒内序列溢出
            if timestamp_sequence.sequence == 0 {
                //阻塞到下一个毫秒,获得新的时间戳
                timestamp = next_millis(timestamp_sequence.timestamp);
                timestamp_sequence.begin_id = (diff_timestamp(timestamp) << TIMESTAMP_LEFT_SHIFT)
                    | (self.worker_id << WORKER_ID_LEFT_SHIFT);
            }
        }
        //时间戳改变，毫秒内序列重置
        else {
//            sequence = betweenLong(0, 1, true);//解决雪花算法在1ms内没有并发尾数始终是偶数问题，但是生成速度会比传统的雪花减少50%，1ms内2048次并发支持
            //上面这个地方的逻辑当时想的是要让这个id能尽量按2模散列,单实际这儿不该做这事，应该是外面用idhash再去散列，所以变回来
            timestamp_sequence.sequence = 0;
            timestamp_sequence.begin_id = (diff_timestamp(timestamp) << TIMESTAMP_LEFT_SHIFT)
                | (self.worker_id << WORKER_ID_LEFT_SHIFT);
        }

        //上次生成ID的时间截
        timestamp_sequence.timestamp = timestamp;

        return timestamp_sequence.begin_id + timestamp_sequence.sequence;
    }
}

fn next_millis(last_timestamp: i64) -> i64 {
    let mut timestamp = get_timestamp();
    while timestamp <= last_timestamp {
        timestamp = get_timestamp();
    }
    timestamp
}

fn get_timestamp() -> i64 {
    // let timestamp = (now_utc().unix_timestamp_nanos()
    //         / Nanosecond::per(Millisecond) as i128)
    //         as i64;
    // timestamp
    current_timestamp_ms()
}

fn diff_timestamp(timestamp: i64) -> i64 {
    let diff_timestamp = timestamp - EPOCH;
    if diff_timestamp > MAX_TIME {
        panic!("超出最大时间限制");
    }
    diff_timestamp
}

fn get_generator() -> &'static Snowflake {
    static INSTANCE: OnceCell<Snowflake> = OnceCell::new();

    INSTANCE.get_or_init(|| {
        let worker_id = std::env::var("WORKER_ID").expect("WORKER_ID is not set in .env file").parse::<u16>().expect("WORKER_ID parse error");
        Snowflake {
            worker_id: worker_id as i64 & WORKER_ID_MASK,
            inner: Mutex::new(TimestampSequence::new()),
        }
    })
}

#[must_use]
pub fn new_id() -> i64 {
    get_generator().next_id()
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use pretty_assertions::assert_eq;

    const THREAD_COUNT: usize = 3;
    const LOOP_COUNT: u64 = 1000000;
    #[tokio::test]
    async fn test_next_id() {
        let mut threads = Vec::with_capacity(THREAD_COUNT);
        let snowflake = Arc::new(Snowflake::new(0));
        let mut ids: Vec<i64> = Vec::new();

        for _i in 0..THREAD_COUNT {
            threads.push(tokio::spawn(loop_next_id(snowflake.clone(), LOOP_COUNT)));
        }

        let init_date_time = SystemTime::now();
        for thread in threads {
            let mut r = thread.await.unwrap();
            r.sort();
            ids.extend(r);
        }
        let milliseconds = SystemTime::now().duration_since(init_date_time).unwrap_or_default().as_millis();
        println!("cost:{milliseconds}ms");
        // ids.sort();
        let mut i = 1;
        let mut count = 0;
        while i < ids.len() {
            if ids[i-1] == ids[i] {
                count += 1;
                // println!("{},{}", ids[i-1], ids[i]);
            }
            i+=1;
        }
        println!("总共id:{}, 重复id:{count}", ids.len());
        assert_eq!(count, 0)
    }

    async fn loop_next_id(snowflake: Arc<Snowflake>, count: u64) -> Vec<i64> {
        let mut ids: Vec<i64> = Vec::new();
        for _ in 0..count {
            let id = snowflake.next_id();
            // let id = snowflake.next_id();
            ids.push(id);
            // println!("{id}");
        }
        ids
    }
}



pub fn benchmark(snowflake: Arc<Snowflake>, loop_count: u64) {
    for _ in 0..loop_count {
        snowflake.next_id();
    }
}