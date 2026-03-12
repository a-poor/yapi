use rusqlite::Connection;

const MIGRATION_000: &str = include_str!("../migrations/000.sql");

pub struct DBClient {
    pub conn: Connection,
}

impl DBClient {
    pub fn new(path: Option<&str>) -> anyhow::Result<Self> {
        let conn = match path {
            Some(p) => Connection::open(p)?,
            None => Connection::open_in_memory()?,
        };
        Ok(Self { conn })
    }

    pub fn migrate(&self) -> anyhow::Result<()> {
        let has_migrations: bool = self.conn.query_row(
            "select count(*) > 0 from sqlite_master where type='table' and name='_migrations'",
            [],
            |row| row.get(0),
        )?;

        if !has_migrations {
            self.conn.execute_batch(MIGRATION_000)?;
        }

        // Future migrations:
        // if version < 1 { run migration 1; }
        // if version < 2 { run migration 2; }
        // ...

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrate_creates_tables() {
        let db = DBClient::new(None).expect("failed to create in-memory db");
        db.migrate().expect("migration failed");

        // Check _migrations table has version 0
        let version: i64 = db
            .conn
            .query_row("select version from _migrations", [], |row| row.get(0))
            .expect("failed to query _migrations");
        assert_eq!(version, 0);

        // Check core tables exist
        let tables: Vec<String> = {
            let mut stmt = db
                .conn
                .prepare("select name from sqlite_master where type='table' order by name")
                .unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };

        assert!(tables.contains(&"workspaces".to_string()));
        assert!(tables.contains(&"requests".to_string()));
        assert!(tables.contains(&"environments".to_string()));
        assert!(tables.contains(&"collections".to_string()));
        assert!(tables.contains(&"history".to_string()));
        assert!(tables.contains(&"_migrations".to_string()));
    }
}
