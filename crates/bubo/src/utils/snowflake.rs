use std::sync::Arc;
use once_cell::sync::OnceCell;
use std::thread;
use std::time::{Duration, SystemTime};
use crossbeam::atomic::AtomicCell;

const EPOCH: i64 = 1574784000000; // 起始时间戳 (2019-11-27)
const WORKER_ID_BITS: u8 = 8;          // 机器 ID 占用位数
const SEQUENCE_BITS: u8 = 14;          // 序列号占用位数

const MAX_WORKER_ID: i64 = (1 << WORKER_ID_BITS) - 1;
const SEQUENCE_MASK: i64 = (1 << SEQUENCE_BITS) - 1;

const TIMESTAMP_SHIFT: u32 = (WORKER_ID_BITS + SEQUENCE_BITS) as u32; // 高位时间戳偏移


pub struct Snowflake {
    worker_id: u8,
    state: AtomicCell<u64>, // 聚合状态，高位存储时间戳，低位存储序列号
}

impl Snowflake {
    pub fn new(worker_id: u8) -> Self {
        if worker_id as i64 > MAX_WORKER_ID || worker_id < 0 {
            panic!("Worker ID must be within 0 ~ {}", MAX_WORKER_ID);
        }
        Snowflake {
            worker_id,
            state: AtomicCell::new(0), // 初始化为 0
        }
    }

    pub fn next_id(&self) -> i64 {
        loop {
            // 获取当前的状态
            let current_state = self.state.load();
            let last_timestamp = (current_state >> TIMESTAMP_SHIFT) as i64; // 高 42 位是时间戳
            let sequence = (current_state & SEQUENCE_MASK as u64) as i64;   // 低 12 位是序列号

            // 获取当前时间戳
            let mut timestamp = Self::current_timestamp();
            if timestamp < last_timestamp {
                // 时钟回退，直接返回到未来时间
                timestamp = last_timestamp; // 强制等待当前逻辑时钟消化
            }

            let new_sequence;
            if timestamp == last_timestamp {
                // 同一毫秒内，增加序列号
                new_sequence = (sequence + 1) & SEQUENCE_MASK;
                if new_sequence == 0 {
                    // 如果序列号溢出，等待下一毫秒
                    timestamp = self.wait_for_next_millis(last_timestamp);
                }
            } else {
                // 不同毫秒，重置序列号
                new_sequence = 0;
            }

            // 生成新的状态
            let new_state = ((timestamp as u64) << TIMESTAMP_SHIFT) | new_sequence as u64;

            // CAS 操作尝试更新状态
            if self.state.compare_exchange(current_state, new_state).is_ok() {
                // 更新成功，生成 ID 并返回
                return self.compose_id(timestamp, new_sequence);
            }

            // 如果 CAS 操作失败，说明有并发冲突，重新尝试
            // 如果 CAS 失败，立即重试（无需显式循环，loop 会处理）
        }
    }

    fn compose_id(&self, timestamp: i64, sequence: i64) -> i64 {
        ((timestamp - EPOCH) << TIMESTAMP_SHIFT)
            | ((self.worker_id as i64) << SEQUENCE_BITS)
            | sequence
    }

    /// 在生成 ID 时，线程直接使用 `GLOBAL_TIMESTAMP` 读取值
    fn current_timestamp() -> i64 {
        super::time::current_timestamp_ms()
        // let now = SystemTime::now();
        // let duration = now.duration_since(UNIX_EPOCH).expect("System clock is before 1970!");
        // duration.as_millis() as i64
    }

    // 改进的等待下一毫秒函数，使用智能休眠策略
    fn wait_for_next_millis(&self, last_timestamp: i64) -> i64 {
        let mut timestamp = Self::current_timestamp();

        while timestamp <= last_timestamp {
            // Sleep 100 微秒避免忙等待
            thread::sleep(Duration::from_micros(100));
            timestamp = Self::current_timestamp();
        }
        timestamp
    }
}

fn get_generator() -> &'static Snowflake {
    static INSTANCE: OnceCell<Snowflake> = OnceCell::new();

    INSTANCE.get_or_init(|| {
        let worker_id = std::env::var("WORKER_ID").expect("WORKER_ID is not set in .env file").parse::<u8>().expect("WORKER_ID parse error");
        Snowflake::new(worker_id)
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

    const THREAD_COUNT: usize = 5;
    const LOOP_COUNT: u64 = 2000000;
    #[tokio::test]
    async fn test_next_id() {
        dotenvy::dotenv().ok();
        // 使用多线程，每个线程负责生成大量 ID
        let num_threads = 5;
        let ids_per_thread = 2_000_000; // 每个线程生成 200 万个 ID

        let mut handles = vec![];
        for _ in 0..num_threads {
            handles.push(std::thread::spawn(move || {
                let mut ids = vec![];
                for _ in 0..ids_per_thread {
                    ids.push(new_id());
                }
                ids
            }));
        }

        let mut all_ids = vec![];
        let init_date_time = SystemTime::now();
        for handle in handles {
            all_ids.extend(handle.join().unwrap());
        }
        let milliseconds = SystemTime::now().duration_since(init_date_time).unwrap_or_default().as_millis();
        println!("cost:{milliseconds}ms speed:{} ids/s", num_threads * ids_per_thread / milliseconds*1_000);
        // 检查是否有重复的 ID
        let total_ids = all_ids.len();
        all_ids.sort();
        all_ids.dedup();
        let unique_ids = all_ids.len();

        println!("Generated {} IDs, {} are unique", total_ids, unique_ids);
        assert_eq!(total_ids, unique_ids)
    }
}



pub fn benchmark(snowflake: Arc<Snowflake>, loop_count: u64) {
    for _ in 0..loop_count {
        snowflake.next_id();
    }
}