use limbo::{Database, params};

fn main() -> anyhow::Result<()> {
    let db = Database::open_in_memory()?;
    
    // Test basic statement creation and execution
    let mut stmt = db.prepare("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")?;
    stmt.execute(params![])?;
    
    println!("Limbo API test successful");
    Ok(())
}
