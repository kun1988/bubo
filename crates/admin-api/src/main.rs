use std::time::SystemTime;
use bubo::utils::snowflake::new_id;


#[tokio::main]
async fn main() {
    // admin_api::main().await;
    // 初始化环境变量
    dotenvy::dotenv().ok();
    let _ = new_id();
    // 多线程测试
    let num_threads = 10;
    let ids_per_thread = 10_000_000;

    let mut handles = vec![];

    for _ in 0..num_threads {
        handles.push(tokio::spawn(async move {
            let mut ids = Vec::with_capacity(ids_per_thread);
            for _ in 0..ids_per_thread {
                ids.push(new_id());
            }
            ids
        }));
    }

    let mut all_ids = vec![];
    let init_date_time = SystemTime::now();
    for handle in handles {
        all_ids.extend(handle.await.unwrap());
    }
    let milliseconds = SystemTime::now().duration_since(init_date_time).unwrap_or_default().as_millis();
    println!("cost:{milliseconds}ms speed:{} ids/s", num_threads * ids_per_thread / milliseconds as usize*1_000);

    // 检查重复
    let total_ids = all_ids.len();
    all_ids.sort_unstable();
    all_ids.dedup();
    let unique_ids = all_ids.len();

    println!("Generated {} IDs, {} are unique", total_ids, unique_ids);
}
