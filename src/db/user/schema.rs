use rusqlite::Row;

pub const TABLE_NAME: &str = "user";

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

pub struct User {
    pub id: i64,
    pub name: String,
    #[allow(dead_code)]
    pub password: String,
    pub roles: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl User {
    pub fn projection() -> String {
        [
            Columns::Id,
            Columns::Name,
            Columns::Password,
            Columns::Roles,
            Columns::CreatedAt,
            Columns::UpdatedAt,
            Columns::DeletedAt,
        ]
        .iter()
        .map(Columns::as_str)
        .collect::<Vec<_>>()
        .join(", ")
    }

    pub fn mapper() -> fn(&Row) -> rusqlite::Result<Self> {
        |row: &Row| -> rusqlite::Result<Self> {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                password: row.get(2)?,
                roles: serde_json::from_value(row.get(3)?).unwrap_or_default(),
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                deleted_at: row.get(6)?,
            })
        }
    }
}
