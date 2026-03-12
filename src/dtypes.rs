use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Environment {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub desc: Option<String>,
    pub value: String,
    pub secret: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Collection {
    pub name: String,
    pub desc: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub name: String,
    pub method: String,
}
