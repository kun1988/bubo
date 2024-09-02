use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use rand::rngs::OsRng;
use sea_orm_migration::{prelude::*, schema::{big_integer, boolean, small_integer, string_len, string_len_uniq, timestamp, tiny_integer}};
use sea_orm_migration::sea_orm::TransactionTrait;
use bubo::utils::{sha256_hash, snowflake};

#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 后台用户表
        let table = Table::create().table(AdminUser::Table).if_not_exists()
            .col(big_integer(AdminUser::Id).primary_key().comment("用户id"))
            .col(string_len_uniq(AdminUser::Username, 50).comment("用户名"))
            .col(string_len(AdminUser::NickName, 50).default("").comment("用户昵称"))
            .col(string_len(AdminUser::Email, 255).default("").comment("用户邮箱"))
            .col(string_len(AdminUser::PhoneNumber, 20).default("").comment("手机号码"))
            .col(tiny_integer(AdminUser::Gender).default(0).comment("用户性别（0未知1男2女）"))
            .col(string_len(AdminUser::Password, 255).comment("密码(sha256加密)"))
            .col(tiny_integer(AdminUser::State).default(1).comment("帐号状态（0未知1正常2停用）"))
            .col(boolean(AdminUser::IsDeleted).default(false).comment("是否删除"))
            .col(boolean(AdminUser::IsAdmin).default(false).comment("是否是管理员"))
            .col(string_len(AdminUser::Remark, 100).default("").comment("备注"))
            .col(big_integer(AdminUser::CreatedBy).default(0).comment("创建人"))
            .col(timestamp(AdminUser::CreatedAt).default(Expr::current_timestamp()).comment("创建时间"))
            .col(big_integer(AdminUser::UpdatedBy).default(0).comment("更新人"))
            .col(timestamp(AdminUser::UpdatedAt).default(Expr::current_timestamp()).comment("更新时间"))
            .comment("后台用户表")
            .to_owned();
        manager.create_table(table).await?;
        let index = Index::create()
            .if_not_exists()
            .name("udx_username")
            .table(AdminUser::Table)
            .col(AdminUser::Username)
            .unique()
            .to_owned();
        manager.create_index(index).await?;

        // 后台用户角色表
        let table = Table::create().table(AdminUserRole::Table).if_not_exists()
            .col(big_integer(AdminUserRole::Id).primary_key().comment("主键id"))
            .col(big_integer(AdminUserRole::UserId).comment("用户id"))
            .col(big_integer(AdminUserRole::RoleId).comment("角色id"))
            .col(big_integer(AdminUserRole::CreatedBy).default(0).comment("创建人"))
            .col(timestamp(AdminUserRole::CreatedAt).default(Expr::current_timestamp()).comment("创建时间"))
            .col(big_integer(AdminUserRole::UpdatedBy).default(0).comment("更新人"))
            .col(timestamp(AdminUserRole::UpdatedAt).default(Expr::current_timestamp()).comment("更新时间"))
            .comment("后台用户角色表")
            .to_owned();
        manager.create_table(table).await?;
        let index = Index::create()
            .if_not_exists()
            .name("udx_user_id_role_id")
            .table(AdminUserRole::Table)
            .col(AdminUserRole::UserId)
            .col(AdminUserRole::RoleId)
            .unique()
            .to_owned();
        manager.create_index(index).await?;

        // 后台角色表
        let table = Table::create().table(AdminRole::Table).if_not_exists()
            .col(big_integer(AdminRole::Id).primary_key().comment("角色id"))
            .col(string_len(AdminRole::Name, 50).comment("角色名称"))
            .col(string_len(AdminRole::Code, 50).unique_key().comment("角色编码"))
            .col(tiny_integer(AdminRole::State).default(1).comment("角色状态（0未知1正常2停用）"))
            .col(small_integer(AdminRole::DisplayOrder).default(1).comment("显示顺序"))
            .col(string_len(AdminRole::Remark, 100).default("").comment("备注"))
            .col(big_integer(AdminRole::CreatedBy).default(0).comment("创建人"))
            .col(timestamp(AdminRole::CreatedAt).default(Expr::current_timestamp()).comment("创建时间"))
            .col(big_integer(AdminRole::UpdatedBy).default(0).comment("更新人"))
            .col(timestamp(AdminRole::UpdatedAt).default(Expr::current_timestamp()).comment("更新时间"))
            .comment("后台角色表")
            .to_owned();
        manager.create_table(table).await?;
        let index = Index::create()
            .if_not_exists()
            .name("udx_code")
            .table(AdminRole::Table)
            .col(AdminRole::Code)
            .unique()
            .to_owned();
        manager.create_index(index).await?;

        // 后台角色菜单表
        let table = Table::create().table(AdminRoleMenu::Table).if_not_exists()
            .col(big_integer(AdminRoleMenu::Id).primary_key().comment("主键id"))
            .col(big_integer(AdminRoleMenu::RoleId).comment("角色id"))
            .col(big_integer(AdminRoleMenu::MenuId).comment("菜单id"))
            .col(big_integer(AdminRoleMenu::CreatedBy).default(0).comment("创建人"))
            .col(timestamp(AdminRoleMenu::CreatedAt).default(Expr::current_timestamp()).comment("创建时间"))
            .col(big_integer(AdminRoleMenu::UpdatedBy).default(0).comment("更新人"))
            .col(timestamp(AdminRoleMenu::UpdatedAt).default(Expr::current_timestamp()).comment("更新时间"))
            .comment("后台角色菜单表")
            .to_owned();
        manager.create_table(table).await?;
        let index = Index::create()
            .if_not_exists()
            .name("udx_role_id_menu_id")
            .table(AdminRoleMenu::Table)
            .col(AdminRoleMenu::RoleId)
            .col(AdminRoleMenu::MenuId)
            .unique()
            .to_owned();
        manager.create_index(index).await?;

        // 后台菜单表
        let table = Table::create().table(AdminMenu::Table).if_not_exists()
            .col(big_integer(AdminMenu::Id).primary_key().comment("菜单id"))
            .col(string_len(AdminMenu::Name, 50).default("").comment("菜单名称"))
            .col(big_integer(AdminMenu::ParentId).default(0).comment("父菜单id"))
            .col(string_len(AdminMenu::Url, 255).default("").comment("请求地址"))
            .col(tiny_integer(AdminMenu::Target).default(1).comment("打开方式（0未知 1页签 2新窗口）"))
            .col(tiny_integer(AdminMenu::MenuType).default(1).comment("菜单类型（0未知 1目录 2菜单 3按钮）"))
            .col(boolean(AdminMenu::IsHidden).default(false).comment("是否隐藏菜单状态"))
            .col(boolean(AdminMenu::IsRefresh).default(false).comment("是否刷新页面"))
            .col(string_len(AdminMenu::Permission, 100).default("").comment("权限标识"))
            .col(string_len(AdminMenu::Icon, 100).default("").comment("菜单图标"))
            .col(small_integer(AdminMenu::DisplayOrder).default(1).comment("显示顺序"))
            .col(string_len(AdminMenu::Remark, 100).default("").comment("备注"))
            .col(big_integer(AdminMenu::CreatedBy).default(0).comment("创建人"))
            .col(timestamp(AdminMenu::CreatedAt).default(Expr::current_timestamp()).comment("创建时间"))
            .col(big_integer(AdminMenu::UpdatedBy).default(0).comment("更新人"))
            .col(timestamp(AdminMenu::UpdatedAt).default(Expr::current_timestamp()).comment("更新时间"))
            .comment("后台菜单表")
            .to_owned();
        manager.create_table(table).await?;

        let pwd = sha256_hash("123456");
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(pwd.as_bytes(), &salt).map_err(|e| DbErr::Custom(e.to_string()))?.to_string();

        // Seeding Data Transactionally
        let db = manager.get_connection();
        let txn = db.begin().await?;

        let insert = Query::insert()
            .into_table(AdminUser::Table)
            .columns([AdminUser::Id, AdminUser::Username, AdminUser::NickName, AdminUser::Password, AdminUser::IsAdmin, AdminUser::Remark])
            .values_panic([snowflake::new_id().into(), "admin".into(), "管理员".into(), password_hash.into(), true.into(), "系统生成".into()])
            .to_owned();
        txn.execute_unprepared(&insert.to_string(PostgresQueryBuilder)).await?;

        txn.commit().await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(AdminUser::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(AdminUserRole::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(AdminRole::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(AdminRoleMenu::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(AdminMenu::Table).to_owned()).await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum AdminUser {
    Table,
    Id,
    Username,
    NickName,
    Email,
    PhoneNumber,
    Gender,
    Password,
    State,
    IsDeleted,
    IsAdmin,
    Remark,
    CreatedBy,
    CreatedAt,
    UpdatedBy,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum AdminUserRole {
    Table,
    Id,
    UserId,
    RoleId,
    CreatedBy,
    CreatedAt,
    UpdatedBy,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum AdminRole {
    Table,
    Id,
    Name,
    Code,
    State,
    DisplayOrder,
    Remark,
    CreatedBy,
    CreatedAt,
    UpdatedBy,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum AdminRoleMenu {
    Table,
    Id,
    RoleId,
    MenuId,
    CreatedBy,
    CreatedAt,
    UpdatedBy,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum AdminMenu {
    Table,
    Id,
    Name,
    ParentId,
    Url,
    Target,
    MenuType,
    IsHidden,
    IsRefresh,
    Permission,
    Icon,
    DisplayOrder,
    Remark,
    CreatedBy,
    CreatedAt,
    UpdatedBy,
    UpdatedAt,
}

