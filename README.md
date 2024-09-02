# bubo
Bubo bubo

数据迁移：

初始化
sea-orm-cli migrate init -d ./crates/admin-migration

创建
sea-orm-cli migrate generate MIGRATION_NAME -d ./crates/admin-migration

构建
sea-orm-cli migrate up -d ./crates/admin-migration
或
cargo run -p admin-migration -- up

生成实体
sea-orm-cli generate entity -o ./crates/admin-api/src/models/_entities --date-time-crate time