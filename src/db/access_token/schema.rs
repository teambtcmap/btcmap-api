pub const NAME: &str = "access_token";

pub enum Columns {
    Id,
    UserId,
    Name,
    Secret,
    Roles,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::UserId => "user_id",
            Columns::Name => "name",
            Columns::Secret => "secret",
            Columns::Roles => "roles",
            Columns::CreatedAt => "created_at",
            Columns::UpdatedAt => "updated_at",
            Columns::DeletedAt => "deleted_at",
        }
    }
}
