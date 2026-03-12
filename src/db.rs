use rusqlite::Connection;

pub struct DBClient {
    pub conn: Connection,
}
