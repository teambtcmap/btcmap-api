use crate::Connection;
use crate::Result;
use crate::lint;

pub fn run(conn: Connection) -> Result<()> {
    lint::generate_issues(&conn)?;
    let report = lint::generate_report(&conn)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
