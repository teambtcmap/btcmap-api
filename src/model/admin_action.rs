// use serde_json::Map;
// use serde_json::Value;

// pub struct AdminAction {
//     pub id: i64,
//     pub user_id: i64,
//     pub message: String,
//     pub tags: Map<String, Value>,
//     pub created_at: String,
//     pub updated_at: String,
//     pub deleted_at: String,
// }

pub static INSERT: &str = r#"
    INSERT INTO admin_action (
        user_id,
        message
    ) VALUES (
        :user_id,
        :message
    )
"#;
