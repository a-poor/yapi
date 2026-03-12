use chrono::NaiveDateTime;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<HeaderEntry>,
    pub body: Option<String>,
    pub duration_secs: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub req_id: Option<i64>,
    pub method: String,
    pub resolved_url: String,
    pub resolved_req_headers: String,
    pub resolved_req_body: Option<String>,
    pub success: bool,
    pub res_status: Option<u16>,
    pub res_body: Option<String>,
    pub res_headers: String,
    pub res_duration: Option<f64>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

pub struct CreateHistoryEntry {
    pub req_id: Option<i64>,
    pub method: String,
    pub resolved_url: String,
    pub resolved_req_headers: String,
    pub resolved_req_body: Option<String>,
    pub success: bool,
    pub res_status: Option<u16>,
    pub res_body: Option<String>,
    pub res_headers: String,
    pub res_duration: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub default_env: Option<i64>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Environment {
    pub id: i64,
    pub workspace_id: i64,
    pub name: String,
    pub description: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Variable {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub value: String,
    pub is_secret: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Collection {
    pub id: i64,
    pub workspace_id: i64,
    pub name: String,
    pub description: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
    CONNECT,
    PATCH,
    TRACE,
}

impl Method {
    pub fn as_str(&self) -> &'static str {
        match self {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::PUT => "PUT",
            Method::DELETE => "DELETE",
            Method::HEAD => "HEAD",
            Method::OPTIONS => "OPTIONS",
            Method::CONNECT => "CONNECT",
            Method::PATCH => "PATCH",
            Method::TRACE => "TRACE",
        }
    }
}

impl<'a> TryFrom<&'a str> for Method {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            "PUT" => Ok(Method::PUT),
            "DELETE" => Ok(Method::DELETE),
            "HEAD" => Ok(Method::HEAD),
            "OPTIONS" => Ok(Method::OPTIONS),
            "CONNECT" => Ok(Method::CONNECT),
            "PATCH" => Ok(Method::PATCH),
            "TRACE" => Ok(Method::TRACE),
            _ => Err("unknown/unsupported method"),
        }
    }
}

impl FromSql for Method {
    fn column_result(value: ValueRef) -> FromSqlResult<Self> {
        let s = String::column_result(value)?;
        Method::try_from(s.as_str()).map_err(|_| FromSqlError::InvalidType)
    }
}

impl ToSql for Method {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(self.as_str().into())
    }
}

impl From<&Method> for reqwest::Method {
    fn from(m: &Method) -> Self {
        match m {
            Method::GET => reqwest::Method::GET,
            Method::POST => reqwest::Method::POST,
            Method::PUT => reqwest::Method::PUT,
            Method::DELETE => reqwest::Method::DELETE,
            Method::HEAD => reqwest::Method::HEAD,
            Method::OPTIONS => reqwest::Method::OPTIONS,
            Method::CONNECT => reqwest::Method::CONNECT,
            Method::PATCH => reqwest::Method::PATCH,
            Method::TRACE => reqwest::Method::TRACE,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub id: i64,
    pub coll_id: i64,
    pub name: String,
    pub method: Method,
    pub url: String,
    pub body: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestHeader {
    pub id: i64,
    pub req_id: i64,
    pub hkey: String,
    pub hval: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestQueryParam {
    pub id: i64,
    pub req_id: i64,
    pub qkey: String,
    pub qval: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
