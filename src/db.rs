use anyhow::Result;
use rusqlite::Connection;

use crate::dtypes::*;

const MIGRATION_000: &str = include_str!("../migrations/000.sql");

#[derive(Debug)]
pub struct NotFoundError {
    pub entity: &'static str,
    pub id: i64,
}

impl std::fmt::Display for NotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} with id {} not found", self.entity, self.id)
    }
}

impl std::error::Error for NotFoundError {}

pub struct DBClient {
    pub conn: Connection,
}

impl DBClient {
    pub fn new(path: Option<&str>) -> Result<Self> {
        let conn = match path {
            Some(p) => Connection::open(p)?,
            None => Connection::open_in_memory()?,
        };
        Ok(Self { conn })
    }

    pub fn migrate(&self) -> Result<()> {
        let has_migrations: bool = self.conn.query_row(
            "select count(*) > 0 from sqlite_master where type='table' and name='_migrations'",
            [],
            |row| row.get(0),
        )?;

        if !has_migrations {
            self.conn.execute_batch(MIGRATION_000)?;
        }

        Ok(())
    }

    // ── Workspaces ──────────────────────────────────────────────

    pub fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, created_at, updated_at FROM workspaces ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Workspace {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_workspace_by_id(&self, id: i64) -> Result<Option<Workspace>> {
        match self.conn.query_row(
            "SELECT id, name, description, created_at, updated_at FROM workspaces WHERE id = ?1",
            [id],
            |row| {
                Ok(Workspace {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        ) {
            Ok(w) => Ok(Some(w)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_workspace_by_name(&self, name: &str) -> Result<Option<Workspace>> {
        match self.conn.query_row(
            "SELECT id, name, description, created_at, updated_at FROM workspaces WHERE name = ?1",
            [name],
            |row| {
                Ok(Workspace {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        ) {
            Ok(w) => Ok(Some(w)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn create_workspace(&self, name: &str, description: &str) -> Result<Workspace> {
        self.conn.execute(
            "INSERT INTO workspaces (name, description) VALUES (?1, ?2)",
            rusqlite::params![name, description],
        )?;
        self.get_workspace_by_id(self.conn.last_insert_rowid())?
            .ok_or_else(|| anyhow::anyhow!("failed to retrieve newly created workspace"))
    }

    pub fn update_workspace(&self, id: i64, name: &str, description: &str) -> Result<()> {
        let rows = self.conn.execute(
            "UPDATE workspaces SET name = ?1, description = ?2, updated_at = datetime('subsec') WHERE id = ?3",
            rusqlite::params![name, description, id],
        )?;
        if rows == 0 {
            return Err(NotFoundError {
                entity: "workspace",
                id,
            }
            .into());
        }
        Ok(())
    }

    pub fn delete_workspace(&self, id: i64) -> Result<()> {
        let rows = self
            .conn
            .execute("DELETE FROM workspaces WHERE id = ?1", [id])?;
        if rows == 0 {
            return Err(NotFoundError {
                entity: "workspace",
                id,
            }
            .into());
        }
        Ok(())
    }

    // ── Environments ────────────────────────────────────────────

    pub fn list_environments(&self, workspace_id: i64) -> Result<Vec<Environment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, workspace_id, name, description, created_at, updated_at FROM environments WHERE workspace_id = ?1 ORDER BY name",
        )?;
        let rows = stmt.query_map([workspace_id], |row| {
            Ok(Environment {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_environment_by_id(&self, id: i64) -> Result<Option<Environment>> {
        match self.conn.query_row(
            "SELECT id, workspace_id, name, description, created_at, updated_at FROM environments WHERE id = ?1",
            [id],
            |row| {
                Ok(Environment {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        ) {
            Ok(e) => Ok(Some(e)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_environment_by_name(
        &self,
        workspace_id: i64,
        name: &str,
    ) -> Result<Option<Environment>> {
        match self.conn.query_row(
            "SELECT id, workspace_id, name, description, created_at, updated_at FROM environments WHERE workspace_id = ?1 AND name = ?2",
            rusqlite::params![workspace_id, name],
            |row| {
                Ok(Environment {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        ) {
            Ok(e) => Ok(Some(e)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn create_environment(
        &self,
        workspace_id: i64,
        name: &str,
        description: &str,
    ) -> Result<Environment> {
        self.conn.execute(
            "INSERT INTO environments (workspace_id, name, description) VALUES (?1, ?2, ?3)",
            rusqlite::params![workspace_id, name, description],
        )?;
        self.get_environment_by_id(self.conn.last_insert_rowid())?
            .ok_or_else(|| anyhow::anyhow!("failed to retrieve newly created environment"))
    }

    pub fn update_environment(&self, id: i64, name: &str, description: &str) -> Result<()> {
        let rows = self.conn.execute(
            "UPDATE environments SET name = ?1, description = ?2, updated_at = datetime('subsec') WHERE id = ?3",
            rusqlite::params![name, description, id],
        )?;
        if rows == 0 {
            return Err(NotFoundError {
                entity: "environment",
                id,
            }
            .into());
        }
        Ok(())
    }

    pub fn delete_environment(&self, id: i64) -> Result<()> {
        let rows = self
            .conn
            .execute("DELETE FROM environments WHERE id = ?1", [id])?;
        if rows == 0 {
            return Err(NotFoundError {
                entity: "environment",
                id,
            }
            .into());
        }
        Ok(())
    }

    // ── Environment Vars ────────────────────────────────────────

    pub fn list_environment_vars(&self, env_id: i64) -> Result<Vec<Variable>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, value, is_secret, created_at, updated_at FROM environment_vars WHERE env_id = ?1 ORDER BY name",
        )?;
        let rows = stmt.query_map([env_id], |row| {
            Ok(Variable {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                value: row.get(3)?,
                is_secret: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_environment_var_by_id(&self, id: i64) -> Result<Option<Variable>> {
        match self.conn.query_row(
            "SELECT id, name, description, value, is_secret, created_at, updated_at FROM environment_vars WHERE id = ?1",
            [id],
            |row| {
                Ok(Variable {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    value: row.get(3)?,
                    is_secret: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        ) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_environment_var_by_name(&self, env_id: i64, name: &str) -> Result<Option<Variable>> {
        match self.conn.query_row(
            "SELECT id, name, description, value, is_secret, created_at, updated_at FROM environment_vars WHERE env_id = ?1 AND name = ?2",
            rusqlite::params![env_id, name],
            |row| {
                Ok(Variable {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    value: row.get(3)?,
                    is_secret: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        ) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn create_environment_var(
        &self,
        env_id: i64,
        name: &str,
        value: &str,
        is_secret: bool,
    ) -> Result<Variable> {
        self.conn.execute(
            "INSERT INTO environment_vars (env_id, name, value, is_secret) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![env_id, name, value, is_secret],
        )?;
        self.get_environment_var_by_id(self.conn.last_insert_rowid())?
            .ok_or_else(|| anyhow::anyhow!("failed to retrieve newly created environment var"))
    }

    pub fn update_environment_var(
        &self,
        id: i64,
        name: &str,
        value: &str,
        is_secret: bool,
    ) -> Result<()> {
        let rows = self.conn.execute(
            "UPDATE environment_vars SET name = ?1, value = ?2, is_secret = ?3, updated_at = datetime('subsec') WHERE id = ?4",
            rusqlite::params![name, value, is_secret, id],
        )?;
        if rows == 0 {
            return Err(NotFoundError {
                entity: "environment_var",
                id,
            }
            .into());
        }
        Ok(())
    }

    pub fn delete_environment_var(&self, id: i64) -> Result<()> {
        let rows = self
            .conn
            .execute("DELETE FROM environment_vars WHERE id = ?1", [id])?;
        if rows == 0 {
            return Err(NotFoundError {
                entity: "environment_var",
                id,
            }
            .into());
        }
        Ok(())
    }

    // ── Collections ─────────────────────────────────────────────

    pub fn list_collections(&self, workspace_id: i64) -> Result<Vec<Collection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, workspace_id, name, description, default_env, created_at, updated_at FROM collections WHERE workspace_id = ?1 ORDER BY name",
        )?;
        let rows = stmt.query_map([workspace_id], |row| {
            Ok(Collection {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                default_env: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_collection_by_id(&self, id: i64) -> Result<Option<Collection>> {
        match self.conn.query_row(
            "SELECT id, workspace_id, name, description, default_env, created_at, updated_at FROM collections WHERE id = ?1",
            [id],
            |row| {
                Ok(Collection {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    default_env: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        ) {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_collection_by_name(
        &self,
        workspace_id: i64,
        name: &str,
    ) -> Result<Option<Collection>> {
        match self.conn.query_row(
            "SELECT id, workspace_id, name, description, default_env, created_at, updated_at FROM collections WHERE workspace_id = ?1 AND name = ?2",
            rusqlite::params![workspace_id, name],
            |row| {
                Ok(Collection {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    default_env: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        ) {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn create_collection(
        &self,
        workspace_id: i64,
        name: &str,
        description: &str,
    ) -> Result<Collection> {
        self.conn.execute(
            "INSERT INTO collections (workspace_id, name, description) VALUES (?1, ?2, ?3)",
            rusqlite::params![workspace_id, name, description],
        )?;
        self.get_collection_by_id(self.conn.last_insert_rowid())?
            .ok_or_else(|| anyhow::anyhow!("failed to retrieve newly created collection"))
    }

    pub fn update_collection(&self, id: i64, name: &str, description: &str) -> Result<()> {
        let rows = self.conn.execute(
            "UPDATE collections SET name = ?1, description = ?2, updated_at = datetime('subsec') WHERE id = ?3",
            rusqlite::params![name, description, id],
        )?;
        if rows == 0 {
            return Err(NotFoundError {
                entity: "collection",
                id,
            }
            .into());
        }
        Ok(())
    }

    pub fn set_collection_default_env(&self, id: i64, env_id: Option<i64>) -> Result<()> {
        let rows = self.conn.execute(
            "UPDATE collections SET default_env = ?1, updated_at = datetime('subsec') WHERE id = ?2",
            rusqlite::params![env_id, id],
        )?;
        if rows == 0 {
            return Err(NotFoundError {
                entity: "collection",
                id,
            }
            .into());
        }
        Ok(())
    }

    pub fn delete_collection(&self, id: i64) -> Result<()> {
        let rows = self
            .conn
            .execute("DELETE FROM collections WHERE id = ?1", [id])?;
        if rows == 0 {
            return Err(NotFoundError {
                entity: "collection",
                id,
            }
            .into());
        }
        Ok(())
    }

    // ── Collection Vars ─────────────────────────────────────────

    pub fn list_collection_vars(&self, coll_id: i64) -> Result<Vec<Variable>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, value, is_secret, created_at, updated_at FROM collection_vars WHERE coll_id = ?1 ORDER BY name",
        )?;
        let rows = stmt.query_map([coll_id], |row| {
            Ok(Variable {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                value: row.get(3)?,
                is_secret: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_collection_var_by_id(&self, id: i64) -> Result<Option<Variable>> {
        match self.conn.query_row(
            "SELECT id, name, description, value, is_secret, created_at, updated_at FROM collection_vars WHERE id = ?1",
            [id],
            |row| {
                Ok(Variable {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    value: row.get(3)?,
                    is_secret: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        ) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_collection_var_by_name(&self, coll_id: i64, name: &str) -> Result<Option<Variable>> {
        match self.conn.query_row(
            "SELECT id, name, description, value, is_secret, created_at, updated_at FROM collection_vars WHERE coll_id = ?1 AND name = ?2",
            rusqlite::params![coll_id, name],
            |row| {
                Ok(Variable {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    value: row.get(3)?,
                    is_secret: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        ) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn create_collection_var(
        &self,
        coll_id: i64,
        name: &str,
        value: &str,
        is_secret: bool,
    ) -> Result<Variable> {
        self.conn.execute(
            "INSERT INTO collection_vars (coll_id, name, value, is_secret) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![coll_id, name, value, is_secret],
        )?;
        self.get_collection_var_by_id(self.conn.last_insert_rowid())?
            .ok_or_else(|| anyhow::anyhow!("failed to retrieve newly created collection var"))
    }

    pub fn update_collection_var(
        &self,
        id: i64,
        name: &str,
        value: &str,
        is_secret: bool,
    ) -> Result<()> {
        let rows = self.conn.execute(
            "UPDATE collection_vars SET name = ?1, value = ?2, is_secret = ?3, updated_at = datetime('subsec') WHERE id = ?4",
            rusqlite::params![name, value, is_secret, id],
        )?;
        if rows == 0 {
            return Err(NotFoundError {
                entity: "collection_var",
                id,
            }
            .into());
        }
        Ok(())
    }

    pub fn delete_collection_var(&self, id: i64) -> Result<()> {
        let rows = self
            .conn
            .execute("DELETE FROM collection_vars WHERE id = ?1", [id])?;
        if rows == 0 {
            return Err(NotFoundError {
                entity: "collection_var",
                id,
            }
            .into());
        }
        Ok(())
    }

    // ── Requests ────────────────────────────────────────────────

    pub fn list_requests(&self, coll_id: i64) -> Result<Vec<Request>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, coll_id, name, method, url, body, created_at, updated_at FROM requests WHERE coll_id = ?1 ORDER BY name",
        )?;
        let rows = stmt.query_map([coll_id], |row| {
            Ok(Request {
                id: row.get(0)?,
                coll_id: row.get(1)?,
                name: row.get(2)?,
                method: row.get(3)?,
                url: row.get(4)?,
                body: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_request_by_id(&self, id: i64) -> Result<Option<Request>> {
        match self.conn.query_row(
            "SELECT id, coll_id, name, method, url, body, created_at, updated_at FROM requests WHERE id = ?1",
            [id],
            |row| {
                Ok(Request {
                    id: row.get(0)?,
                    coll_id: row.get(1)?,
                    name: row.get(2)?,
                    method: row.get(3)?,
                    url: row.get(4)?,
                    body: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        ) {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_request_by_name(&self, coll_id: i64, name: &str) -> Result<Option<Request>> {
        match self.conn.query_row(
            "SELECT id, coll_id, name, method, url, body, created_at, updated_at FROM requests WHERE coll_id = ?1 AND name = ?2",
            rusqlite::params![coll_id, name],
            |row| {
                Ok(Request {
                    id: row.get(0)?,
                    coll_id: row.get(1)?,
                    name: row.get(2)?,
                    method: row.get(3)?,
                    url: row.get(4)?,
                    body: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        ) {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn create_request(
        &self,
        coll_id: i64,
        name: &str,
        method: &str,
        url: &str,
        body: Option<&str>,
    ) -> Result<Request> {
        self.conn.execute(
            "INSERT INTO requests (coll_id, name, method, url, body) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![coll_id, name, method, url, body],
        )?;
        self.get_request_by_id(self.conn.last_insert_rowid())?
            .ok_or_else(|| anyhow::anyhow!("failed to retrieve newly created request"))
    }

    pub fn update_request(
        &self,
        id: i64,
        name: &str,
        method: &str,
        url: &str,
        body: Option<&str>,
    ) -> Result<()> {
        let rows = self.conn.execute(
            "UPDATE requests SET name = ?1, method = ?2, url = ?3, body = ?4, updated_at = datetime('subsec') WHERE id = ?5",
            rusqlite::params![name, method, url, body, id],
        )?;
        if rows == 0 {
            return Err(NotFoundError {
                entity: "request",
                id,
            }
            .into());
        }
        Ok(())
    }

    pub fn delete_request(&self, id: i64) -> Result<()> {
        let rows = self
            .conn
            .execute("DELETE FROM requests WHERE id = ?1", [id])?;
        if rows == 0 {
            return Err(NotFoundError {
                entity: "request",
                id,
            }
            .into());
        }
        Ok(())
    }

    // ── Request Headers ─────────────────────────────────────────

    pub fn list_request_headers(&self, req_id: i64) -> Result<Vec<RequestHeader>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, req_id, hkey, hval, created_at, updated_at FROM request_headers WHERE req_id = ?1",
        )?;
        let rows = stmt.query_map([req_id], |row| {
            Ok(RequestHeader {
                id: row.get(0)?,
                req_id: row.get(1)?,
                hkey: row.get(2)?,
                hval: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_request_header_by_id(&self, id: i64) -> Result<Option<RequestHeader>> {
        match self.conn.query_row(
            "SELECT id, req_id, hkey, hval, created_at, updated_at FROM request_headers WHERE id = ?1",
            [id],
            |row| {
                Ok(RequestHeader {
                    id: row.get(0)?,
                    req_id: row.get(1)?,
                    hkey: row.get(2)?,
                    hval: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        ) {
            Ok(h) => Ok(Some(h)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn create_request_header(
        &self,
        req_id: i64,
        hkey: &str,
        hval: &str,
    ) -> Result<RequestHeader> {
        self.conn.execute(
            "INSERT INTO request_headers (req_id, hkey, hval) VALUES (?1, ?2, ?3)",
            rusqlite::params![req_id, hkey, hval],
        )?;
        self.get_request_header_by_id(self.conn.last_insert_rowid())?
            .ok_or_else(|| anyhow::anyhow!("failed to retrieve newly created request header"))
    }

    pub fn update_request_header(&self, id: i64, hkey: &str, hval: &str) -> Result<()> {
        let rows = self.conn.execute(
            "UPDATE request_headers SET hkey = ?1, hval = ?2, updated_at = datetime('subsec') WHERE id = ?3",
            rusqlite::params![hkey, hval, id],
        )?;
        if rows == 0 {
            return Err(NotFoundError {
                entity: "request_header",
                id,
            }
            .into());
        }
        Ok(())
    }

    pub fn delete_request_header(&self, id: i64) -> Result<()> {
        let rows = self
            .conn
            .execute("DELETE FROM request_headers WHERE id = ?1", [id])?;
        if rows == 0 {
            return Err(NotFoundError {
                entity: "request_header",
                id,
            }
            .into());
        }
        Ok(())
    }

    // ── Request Query Params ────────────────────────────────────

    pub fn list_request_query_params(&self, req_id: i64) -> Result<Vec<RequestQueryParam>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, req_id, qkey, qval, created_at, updated_at FROM request_query_params WHERE req_id = ?1",
        )?;
        let rows = stmt.query_map([req_id], |row| {
            Ok(RequestQueryParam {
                id: row.get(0)?,
                req_id: row.get(1)?,
                qkey: row.get(2)?,
                qval: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_request_query_param_by_id(&self, id: i64) -> Result<Option<RequestQueryParam>> {
        match self.conn.query_row(
            "SELECT id, req_id, qkey, qval, created_at, updated_at FROM request_query_params WHERE id = ?1",
            [id],
            |row| {
                Ok(RequestQueryParam {
                    id: row.get(0)?,
                    req_id: row.get(1)?,
                    qkey: row.get(2)?,
                    qval: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        ) {
            Ok(q) => Ok(Some(q)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn create_request_query_param(
        &self,
        req_id: i64,
        qkey: &str,
        qval: &str,
    ) -> Result<RequestQueryParam> {
        self.conn.execute(
            "INSERT INTO request_query_params (req_id, qkey, qval) VALUES (?1, ?2, ?3)",
            rusqlite::params![req_id, qkey, qval],
        )?;
        self.get_request_query_param_by_id(self.conn.last_insert_rowid())?
            .ok_or_else(|| anyhow::anyhow!("failed to retrieve newly created request query param"))
    }

    pub fn update_request_query_param(&self, id: i64, qkey: &str, qval: &str) -> Result<()> {
        let rows = self.conn.execute(
            "UPDATE request_query_params SET qkey = ?1, qval = ?2, updated_at = datetime('subsec') WHERE id = ?3",
            rusqlite::params![qkey, qval, id],
        )?;
        if rows == 0 {
            return Err(NotFoundError {
                entity: "request_query_param",
                id,
            }
            .into());
        }
        Ok(())
    }

    pub fn delete_request_query_param(&self, id: i64) -> Result<()> {
        let rows = self
            .conn
            .execute("DELETE FROM request_query_params WHERE id = ?1", [id])?;
        if rows == 0 {
            return Err(NotFoundError {
                entity: "request_query_param",
                id,
            }
            .into());
        }
        Ok(())
    }
    // ── History ─────────────────────────────────────────────────

    pub fn list_history(&self, req_id: i64) -> Result<Vec<HistoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, req_id, method, resolved_url, resolved_req_headers, resolved_req_body, success, res_status, res_body, res_headers, res_duration, created_at, updated_at FROM history WHERE req_id = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([req_id], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                req_id: row.get(1)?,
                method: row.get(2)?,
                resolved_url: row.get(3)?,
                resolved_req_headers: row.get(4)?,
                resolved_req_body: row.get(5)?,
                success: row.get(6)?,
                res_status: row.get(7)?,
                res_body: row.get(8)?,
                res_headers: row.get(9)?,
                res_duration: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_history_by_id(&self, id: i64) -> Result<Option<HistoryEntry>> {
        match self.conn.query_row(
            "SELECT id, req_id, method, resolved_url, resolved_req_headers, resolved_req_body, success, res_status, res_body, res_headers, res_duration, created_at, updated_at FROM history WHERE id = ?1",
            [id],
            |row| {
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    req_id: row.get(1)?,
                    method: row.get(2)?,
                    resolved_url: row.get(3)?,
                    resolved_req_headers: row.get(4)?,
                    resolved_req_body: row.get(5)?,
                    success: row.get(6)?,
                    res_status: row.get(7)?,
                    res_body: row.get(8)?,
                    res_headers: row.get(9)?,
                    res_duration: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            },
        ) {
            Ok(h) => Ok(Some(h)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn create_history(&self, entry: &CreateHistoryEntry) -> Result<HistoryEntry> {
        self.conn.execute(
            "INSERT INTO history (req_id, method, resolved_url, resolved_req_headers, resolved_req_body, success, res_status, res_body, res_headers, res_duration) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                entry.req_id,
                entry.method,
                entry.resolved_url,
                entry.resolved_req_headers,
                entry.resolved_req_body,
                entry.success,
                entry.res_status,
                entry.res_body,
                entry.res_headers,
                entry.res_duration,
            ],
        )?;
        self.get_history_by_id(self.conn.last_insert_rowid())?
            .ok_or_else(|| anyhow::anyhow!("failed to retrieve newly created history entry"))
    }

    pub fn delete_history(&self, id: i64) -> Result<()> {
        let rows = self
            .conn
            .execute("DELETE FROM history WHERE id = ?1", [id])?;
        if rows == 0 {
            return Err(NotFoundError {
                entity: "history",
                id,
            }
            .into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> DBClient {
        let db = DBClient::new(None).expect("failed to create in-memory db");
        db.migrate().expect("migration failed");
        db
    }

    #[test]
    fn test_migrate_creates_tables() {
        let db = setup();

        let version: i64 = db
            .conn
            .query_row("select version from _migrations", [], |row| row.get(0))
            .expect("failed to query _migrations");
        assert_eq!(version, 0);

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

    #[test]
    fn test_workspace_crud() {
        let db = setup();

        // The migration seeds a "default" workspace
        let workspaces = db.list_workspaces().unwrap();
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].name, "default");

        // Create
        let ws = db.create_workspace("test-ws", "a test workspace").unwrap();
        assert_eq!(ws.name, "test-ws");
        assert_eq!(ws.description, "a test workspace");

        // Get by name
        let ws2 = db
            .get_workspace_by_name("test-ws")
            .unwrap()
            .expect("workspace not found");
        assert_eq!(ws2.id, ws.id);

        // Get by id
        let ws3 = db
            .get_workspace_by_id(ws.id)
            .unwrap()
            .expect("workspace not found");
        assert_eq!(ws3.name, "test-ws");

        // Get non-existent returns None
        assert!(db.get_workspace_by_id(9999).unwrap().is_none());
        assert!(db.get_workspace_by_name("no-such-ws").unwrap().is_none());

        // List
        let workspaces = db.list_workspaces().unwrap();
        assert_eq!(workspaces.len(), 2);

        // Update
        db.update_workspace(ws.id, "renamed-ws", "updated desc")
            .unwrap();
        let ws4 = db
            .get_workspace_by_id(ws.id)
            .unwrap()
            .expect("workspace not found");
        assert_eq!(ws4.name, "renamed-ws");
        assert_eq!(ws4.description, "updated desc");

        // Update non-existent returns NotFoundError
        let err = db.update_workspace(9999, "x", "y").unwrap_err();
        assert!(err.downcast_ref::<NotFoundError>().is_some());

        // Delete
        db.delete_workspace(ws.id).unwrap();
        let workspaces = db.list_workspaces().unwrap();
        assert_eq!(workspaces.len(), 1);

        // Delete non-existent returns NotFoundError
        let err = db.delete_workspace(ws.id).unwrap_err();
        assert!(err.downcast_ref::<NotFoundError>().is_some());
    }

    #[test]
    fn test_collection_crud() {
        let db = setup();
        let ws = db
            .get_workspace_by_name("default")
            .unwrap()
            .expect("workspace not found");

        // Create
        let coll = db
            .create_collection(ws.id, "my-collection", "desc")
            .unwrap();
        assert_eq!(coll.name, "my-collection");
        assert_eq!(coll.workspace_id, ws.id);

        // Get by name
        let coll2 = db
            .get_collection_by_name(ws.id, "my-collection")
            .unwrap()
            .expect("collection not found");
        assert_eq!(coll2.id, coll.id);

        // Get non-existent returns None
        assert!(db.get_collection_by_id(9999).unwrap().is_none());

        // List
        let colls = db.list_collections(ws.id).unwrap();
        assert_eq!(colls.len(), 1);

        // Update
        db.update_collection(coll.id, "renamed-coll", "new desc")
            .unwrap();
        let coll3 = db
            .get_collection_by_id(coll.id)
            .unwrap()
            .expect("collection not found");
        assert_eq!(coll3.name, "renamed-coll");

        // Update non-existent returns NotFoundError
        let err = db.update_collection(9999, "x", "y").unwrap_err();
        assert!(err.downcast_ref::<NotFoundError>().is_some());

        // Delete
        db.delete_collection(coll.id).unwrap();
        let colls = db.list_collections(ws.id).unwrap();
        assert_eq!(colls.len(), 0);

        // Delete non-existent returns NotFoundError
        let err = db.delete_collection(coll.id).unwrap_err();
        assert!(err.downcast_ref::<NotFoundError>().is_some());
    }

    #[test]
    fn test_request_crud() {
        let db = setup();
        let ws = db
            .get_workspace_by_name("default")
            .unwrap()
            .expect("workspace not found");
        let coll = db.create_collection(ws.id, "coll", "").unwrap();

        // Create
        let req = db
            .create_request(
                coll.id,
                "get-users",
                "GET",
                "https://api.example.com/users",
                None,
            )
            .unwrap();
        assert_eq!(req.name, "get-users");
        assert_eq!(req.method, Method::GET);
        assert!(req.body.is_none());

        // Get by name
        let req2 = db
            .get_request_by_name(coll.id, "get-users")
            .unwrap()
            .expect("request not found");
        assert_eq!(req2.id, req.id);

        // Get non-existent returns None
        assert!(db.get_request_by_id(9999).unwrap().is_none());

        // List
        let reqs = db.list_requests(coll.id).unwrap();
        assert_eq!(reqs.len(), 1);

        // Update with body
        db.update_request(
            req.id,
            "create-user",
            "POST",
            "https://api.example.com/users",
            Some("{\"name\":\"test\"}"),
        )
        .unwrap();
        let req3 = db
            .get_request_by_id(req.id)
            .unwrap()
            .expect("request not found");
        assert_eq!(req3.name, "create-user");
        assert_eq!(req3.method, Method::POST);
        assert_eq!(req3.body.as_deref(), Some("{\"name\":\"test\"}"));

        // Update non-existent returns NotFoundError
        let err = db
            .update_request(9999, "x", "GET", "http://x", None)
            .unwrap_err();
        assert!(err.downcast_ref::<NotFoundError>().is_some());

        // Delete
        db.delete_request(req.id).unwrap();
        let reqs = db.list_requests(coll.id).unwrap();
        assert_eq!(reqs.len(), 0);

        // Delete non-existent returns NotFoundError
        let err = db.delete_request(req.id).unwrap_err();
        assert!(err.downcast_ref::<NotFoundError>().is_some());
    }

    #[test]
    fn test_history_crud() {
        let db = setup();
        let ws = db
            .get_workspace_by_name("default")
            .unwrap()
            .expect("workspace not found");
        let coll = db.create_collection(ws.id, "coll", "").unwrap();
        let req = db
            .create_request(coll.id, "test-req", "GET", "https://example.com", None)
            .unwrap();

        // Create
        let entry = CreateHistoryEntry {
            req_id: Some(req.id),
            method: "GET".to_string(),
            resolved_url: "https://example.com".to_string(),
            resolved_req_headers: "[]".to_string(),
            resolved_req_body: None,
            success: true,
            res_status: Some(200),
            res_body: Some("{\"ok\":true}".to_string()),
            res_headers: "[{\"key\":\"content-type\",\"value\":\"application/json\"}]".to_string(),
            res_duration: Some(0.123),
        };
        let hist = db.create_history(&entry).unwrap();
        assert_eq!(hist.method, "GET");
        assert_eq!(hist.resolved_url, "https://example.com");
        assert_eq!(hist.req_id, Some(req.id));
        assert!(hist.success);
        assert_eq!(hist.res_status, Some(200));
        assert_eq!(hist.res_body.as_deref(), Some("{\"ok\":true}"));
        assert_eq!(hist.res_duration, Some(0.123));

        // Get by id
        let hist2 = db
            .get_history_by_id(hist.id)
            .unwrap()
            .expect("history not found");
        assert_eq!(hist2.id, hist.id);
        assert_eq!(hist2.method, "GET");

        // Get non-existent returns None
        assert!(db.get_history_by_id(9999).unwrap().is_none());

        // List
        let entries = db.list_history(req.id).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, hist.id);

        // Delete
        db.delete_history(hist.id).unwrap();
        let entries = db.list_history(req.id).unwrap();
        assert_eq!(entries.len(), 0);

        // Delete non-existent returns NotFoundError
        let err = db.delete_history(hist.id).unwrap_err();
        assert!(err.downcast_ref::<NotFoundError>().is_some());
    }
}
