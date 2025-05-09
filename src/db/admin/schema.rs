pub const NAME: &str = "admin";

pub enum Columns {
    Id,
    Name,
    Password,
    Roles,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::Name => "name",
            Columns::Password => "password",
            Columns::Roles => "roles",
            Columns::CreatedAt => "created_at",
            Columns::UpdatedAt => "updated_at",
            Columns::DeletedAt => "deleted_at",
        }
    }
}
