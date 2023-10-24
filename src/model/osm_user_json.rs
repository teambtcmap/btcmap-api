use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Serialize, Deserialize)]
pub struct OsmUserJson {
    pub id: i32,
    pub display_name: String,
    #[serde(with = "time::serde::rfc3339")]
    pub account_created: OffsetDateTime,
    pub description: String,
    pub contributor_terms: ContributorTerms,
    pub img: Option<Img>,
    pub roles: Vec<String>,
    pub changesets: Changesets,
    pub traces: Traces,
    pub blocks: Blocks,
}

#[derive(Serialize, Deserialize)]
pub struct ContributorTerms {
    pub agreed: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Img {
    pub href: String,
}

#[derive(Serialize, Deserialize)]
pub struct Changesets {
    pub count: i32,
}

#[derive(Serialize, Deserialize)]
pub struct Traces {
    pub count: i32,
}

#[derive(Serialize, Deserialize)]
pub struct Blocks {
    received: BlocksReceived,
}

#[derive(Serialize, Deserialize)]
pub struct BlocksReceived {
    count: i32,
    active: i32,
}

impl OsmUserJson {
    #[cfg(test)]
    pub fn mock() -> OsmUserJson {
        OsmUserJson {
            id: 1,
            display_name: "".into(),
            account_created: OffsetDateTime::now_utc(),
            description: "".into(),
            contributor_terms: ContributorTerms { agreed: true },
            img: None,
            roles: vec![],
            changesets: Changesets { count: 0 },
            traces: Traces { count: 0 },
            blocks: Blocks {
                received: BlocksReceived {
                    count: 0,
                    active: 0,
                },
            },
        }
    }
}
