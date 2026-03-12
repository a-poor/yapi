use std::collections::HashMap;
use std::fmt;
use std::sync::LazyLock;

use anyhow::Result;
use regex::Regex;

use crate::dtypes::{Request, RequestHeader, RequestQueryParam, Variable};

static VAR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{\s*([\w]+)\s*\}\}").unwrap());

#[derive(Debug)]
pub struct UndefinedVarsError {
    pub names: Vec<String>,
}

impl fmt::Display for UndefinedVarsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "undefined variables: {}", self.names.join(", "))
    }
}

impl std::error::Error for UndefinedVarsError {}

/// Replace `{{ var_name }}` placeholders in `template` with values from `vars`.
/// Returns an error listing all missing variable names.
pub fn fill(template: &str, vars: &HashMap<String, String>) -> Result<String> {
    let mut missing: Vec<String> = Vec::new();

    let result = VAR_RE.replace_all(template, |caps: &regex::Captures| {
        let name = caps[1].to_string();
        match vars.get(&name) {
            Some(val) => val.clone(),
            None => {
                if !missing.contains(&name) {
                    missing.push(name.clone());
                }
                caps[0].to_string()
            }
        }
    });

    if !missing.is_empty() {
        return Err(UndefinedVarsError { names: missing }.into());
    }

    Ok(result.into_owned())
}

/// Build a variable map from collection and environment variables.
/// Environment variables override collection variables on name collision.
pub fn build_var_map(coll_vars: &[Variable], env_vars: &[Variable]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for v in coll_vars {
        map.insert(v.name.clone(), v.value.clone());
    }
    for v in env_vars {
        map.insert(v.name.clone(), v.value.clone());
    }
    map
}

pub struct ResolvedRequest {
    pub url: String,
    pub body: Option<String>,
    pub headers: Vec<(String, String)>,
    pub query_params: Vec<(String, String)>,
}

/// Resolve all template placeholders in a request's fields.
pub fn resolve_request(
    request: &Request,
    headers: &[RequestHeader],
    query_params: &[RequestQueryParam],
    vars: &HashMap<String, String>,
) -> Result<ResolvedRequest> {
    let url = fill(&request.url, vars)?;
    let body = match &request.body {
        Some(b) => Some(fill(b, vars)?),
        None => None,
    };
    let mut resolved_headers = Vec::with_capacity(headers.len());
    for h in headers {
        resolved_headers.push((h.hkey.clone(), fill(&h.hval, vars)?));
    }
    let mut resolved_qp = Vec::with_capacity(query_params.len());
    for qp in query_params {
        resolved_qp.push((qp.qkey.clone(), fill(&qp.qval, vars)?));
    }
    Ok(ResolvedRequest {
        url,
        body,
        headers: resolved_headers,
        query_params: resolved_qp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn make_variable(name: &str, value: &str) -> Variable {
        Variable {
            id: 0,
            name: name.to_string(),
            description: String::new(),
            value: value.to_string(),
            is_secret: false,
            created_at: NaiveDateTime::default(),
            updated_at: NaiveDateTime::default(),
        }
    }

    fn make_request(url: &str, body: Option<&str>) -> Request {
        Request {
            id: 0,
            coll_id: 0,
            name: String::new(),
            method: "GET".to_string(),
            url: url.to_string(),
            body: body.map(|s| s.to_string()),
            created_at: NaiveDateTime::default(),
            updated_at: NaiveDateTime::default(),
        }
    }

    #[test]
    fn basic_substitution() {
        let v = vars(&[("host", "example.com")]);
        assert_eq!(
            fill("https://{{ host }}/api", &v).unwrap(),
            "https://example.com/api"
        );
    }

    #[test]
    fn multiple_vars() {
        let v = vars(&[("host", "example.com"), ("port", "8080")]);
        assert_eq!(
            fill("{{ host }}:{{ port }}", &v).unwrap(),
            "example.com:8080"
        );
    }

    #[test]
    fn repeated_var() {
        let v = vars(&[("x", "hi")]);
        assert_eq!(fill("{{ x }} and {{ x }}", &v).unwrap(), "hi and hi");
    }

    #[test]
    fn whitespace_flexibility() {
        let v = vars(&[("x", "val")]);
        assert_eq!(fill("{{x}}", &v).unwrap(), "val");
        assert_eq!(fill("{{ x }}", &v).unwrap(), "val");
        assert_eq!(fill("{{  x  }}", &v).unwrap(), "val");
    }

    #[test]
    fn no_placeholders_passthrough() {
        let v = vars(&[]);
        assert_eq!(fill("hello world", &v).unwrap(), "hello world");
    }

    #[test]
    fn missing_var_error() {
        let v = vars(&[]);
        let err = fill("{{ missing }}", &v).unwrap_err();
        let undef = err.downcast_ref::<UndefinedVarsError>().unwrap();
        assert_eq!(undef.names, vec!["missing"]);
    }

    #[test]
    fn multiple_missing_vars() {
        let v = vars(&[]);
        let err = fill("{{ a }} {{ b }} {{ a }}", &v).unwrap_err();
        let undef = err.downcast_ref::<UndefinedVarsError>().unwrap();
        assert_eq!(undef.names, vec!["a", "b"]);
    }

    #[test]
    fn partial_missing() {
        let v = vars(&[("exists", "yes")]);
        let err = fill("{{ exists }} {{ nope }}", &v).unwrap_err();
        let undef = err.downcast_ref::<UndefinedVarsError>().unwrap();
        assert_eq!(undef.names, vec!["nope"]);
    }

    #[test]
    fn empty_braces_passthrough() {
        let v = vars(&[]);
        assert_eq!(fill("{{  }}", &v).unwrap(), "{{  }}");
    }

    #[test]
    fn build_var_map_env_overrides_coll() {
        let coll = vec![make_variable("key", "coll_val")];
        let env = vec![make_variable("key", "env_val")];
        let map = build_var_map(&coll, &env);
        assert_eq!(map.get("key").unwrap(), "env_val");
    }

    #[test]
    fn resolve_request_fills_all_fields() {
        let v = vars(&[("host", "example.com"), ("token", "abc")]);
        let req = make_request("https://{{ host }}/api", Some(r#"{"token":"{{ token }}"}"#));
        let headers = vec![RequestHeader {
            id: 0,
            req_id: 0,
            hkey: "Authorization".to_string(),
            hval: "Bearer {{ token }}".to_string(),
            created_at: NaiveDateTime::default(),
            updated_at: NaiveDateTime::default(),
        }];
        let qps = vec![RequestQueryParam {
            id: 0,
            req_id: 0,
            qkey: "host".to_string(),
            qval: "{{ host }}".to_string(),
            created_at: NaiveDateTime::default(),
            updated_at: NaiveDateTime::default(),
        }];
        let resolved = resolve_request(&req, &headers, &qps, &v).unwrap();
        assert_eq!(resolved.url, "https://example.com/api");
        assert_eq!(resolved.body.unwrap(), r#"{"token":"abc"}"#);
        assert_eq!(
            resolved.headers[0],
            ("Authorization".to_string(), "Bearer abc".to_string())
        );
        assert_eq!(
            resolved.query_params[0],
            ("host".to_string(), "example.com".to_string())
        );
    }

    #[test]
    fn resolve_request_none_body_stays_none() {
        let v = vars(&[("host", "example.com")]);
        let req = make_request("https://{{ host }}", None);
        let resolved = resolve_request(&req, &[], &[], &v).unwrap();
        assert!(resolved.body.is_none());
    }
}
