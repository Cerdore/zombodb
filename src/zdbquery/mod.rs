use pgx::*;
use serde_json::{json, Value};

mod cast;
pub mod mvcc;
mod opclass;

use crate::gucs::ZDB_DEFAULT_ROW_ESTIMATE;
pub use pg_catalog::*;
use std::collections::HashMap;
use std::ffi::CStr;

mod pg_catalog {
    #![allow(non_camel_case_types)]
    use pgx::*;
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use std::collections::HashMap;

    #[derive(Debug, Serialize, Deserialize, PostgresType)]
    #[inoutfuncs]
    pub struct ZDBQuery {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub(super) limit: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub(super) offset: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub(super) min_score: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub(super) sort_json: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub(super) row_estimate: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub(super) query_dsl: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub(super) want_score: Option<bool>,
        #[serde(skip_serializing_if = "HashMap::is_empty")]
        #[serde(default = "HashMap::new")]
        pub(super) highlights: HashMap<String, Value>,
    }

    #[derive(PostgresEnum, Serialize, Deserialize)]
    pub enum SortDirection {
        asc,
        desc,
    }

    #[derive(PostgresEnum, Serialize, Deserialize)]
    pub enum SortMode {
        min,
        max,
        sum,
        avg,
        median,
    }

    #[derive(PostgresType, Serialize, Deserialize)]
    pub struct SortDescriptorOptions {
        pub(crate) order: SortDirection,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub(crate) mode: Option<SortMode>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub(crate) nested_path: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub(crate) nested_filter: Option<Value>,
    }

    #[derive(PostgresType, Serialize, Deserialize)]
    pub struct SortDescriptor {
        pub(crate) field: String,
        pub(crate) options: SortDescriptorOptions,
    }
}

impl InOutFuncs for ZDBQuery {
    fn input(input: &CStr) -> Self {
        let input = input.to_str().expect("zdbquery input is not valid UTF8");
        let value: Value = match serde_json::from_str(input) {
            Ok(value) => value,
            Err(_) => {
                // it's not json so assume it's a query_string
                return ZDBQuery::new_with_query_string(input);
            }
        };

        match serde_json::from_value::<ZDBQuery>(value.clone()) {
            Ok(zdbquery) => {
                if zdbquery.all_none() {
                    // it parsed as valid json but didn't match any of our
                    // struct fields, so treat it as if it's query dsl
                    ZDBQuery::new_with_query_dsl(value)
                } else {
                    // it's a real ZDBQuery
                    zdbquery
                }
            }
            Err(_) => ZDBQuery::new_with_query_string(input),
        }
    }

    fn output(&self, buffer: &mut StringInfo)
    where
        Self: serde::ser::Serialize,
    {
        if self.only_query_dsl() {
            serde_json::to_writer(buffer, self.query_dsl().unwrap())
                .expect("failed to serialize a {} to json")
        } else {
            serde_json::to_writer(buffer, self).expect("failed to serialize a {} to json")
        }
    }
}

impl Default for ZDBQuery {
    fn default() -> Self {
        ZDBQuery {
            want_score: None,
            row_estimate: None,
            limit: None,
            offset: None,
            min_score: None,
            sort_json: None,
            query_dsl: None,
            highlights: HashMap::new(),
        }
    }
}

#[allow(dead_code)]
impl ZDBQuery {
    pub fn new_with_query_dsl(query_dsl: Value) -> Self {
        ZDBQuery {
            want_score: None,
            row_estimate: None,
            limit: None,
            offset: None,
            min_score: None,
            sort_json: None,
            query_dsl: Some(query_dsl),
            highlights: HashMap::new(),
        }
    }

    pub fn new_with_query_string(query: &str) -> Self {
        let query_json = if query.trim().is_empty() {
            // an empty query means to match everything
            serde_json::json!({"match_all":{}})
        } else {
            // else it gets turned into an Elasticsearch query_string query
            serde_json::json!({"query_string":{"query": query}})
        };

        ZDBQuery {
            want_score: None,
            row_estimate: None,
            limit: None,
            offset: None,
            min_score: None,
            sort_json: None,
            query_dsl: Some(query_json),
            highlights: HashMap::new(),
        }
    }

    pub fn all_none(&self) -> bool {
        self.want_score.is_none()
            && self.row_estimate.is_none()
            && self.limit.is_none()
            && self.offset.is_none()
            && self.min_score.is_none()
            && self.sort_json.is_none()
            && self.query_dsl.is_none()
            && self.highlights.is_empty()
    }

    pub fn only_query_dsl(&self) -> bool {
        self.query_dsl.is_some()
            && self.want_score.is_none()
            && self.row_estimate.is_none()
            && self.limit.is_none()
            && self.offset.is_none()
            && self.min_score.is_none()
            && self.sort_json.is_none()
            && self.highlights.is_empty()
    }

    pub fn want_score(&self) -> bool {
        self.want_score == Some(true)
    }

    pub fn set_want_score(mut self, value: bool) -> Self {
        self.want_score = if value { Some(true) } else { None };
        self
    }

    pub fn highlights(&mut self) -> &mut HashMap<String, Value> {
        &mut self.highlights
    }

    pub fn row_estimate(&self) -> i64 {
        match self.row_estimate {
            Some(estimate) => estimate,
            None => ZDB_DEFAULT_ROW_ESTIMATE.get() as i64,
        }
    }

    pub fn set_row_estimate(mut self, row_estimate: Option<i64>) -> Self {
        self.row_estimate = row_estimate;
        self
    }

    pub fn limit(&self) -> Option<u64> {
        self.limit
    }

    pub fn set_limit(mut self, limit: Option<u64>) -> Self {
        self.limit = limit;
        self
    }

    pub fn offset(&self) -> Option<u64> {
        self.offset
    }

    pub fn set_offset(mut self, offset: Option<u64>) -> Self {
        self.offset = offset;
        self
    }

    pub fn min_score(&self) -> Option<f64> {
        self.min_score
    }

    pub fn set_min_score(mut self, min_score: Option<f64>) -> Self {
        self.min_score = min_score;
        self
    }

    pub fn sort_json(&self) -> Option<&Value> {
        self.sort_json.as_ref()
    }

    pub fn set_sort_descriptors<I>(mut self, descriptors: I) -> Self
    where
        I: IntoIterator<Item = Option<SortDescriptor>>,
    {
        // collect all non-None (NULL) sort descriptors
        let mut v = Vec::new();
        for descriptor in descriptors {
            if let Some(descriptor) = descriptor {
                v.push(json! {
                    {
                        &descriptor.field: descriptor.options
                    }
                });
            }
        }

        if !v.is_empty() {
            // we have at least one, so serialize it
            self.sort_json = Some(serde_json::to_value(v).unwrap());
        } else {
            // else, we don't have any
            self.sort_json = None;
        }
        self
    }

    pub fn set_sort_json(mut self, sort_json: Option<Value>) -> Self {
        self.sort_json = sort_json;
        self
    }

    pub fn query_dsl(&self) -> Option<&Value> {
        self.query_dsl.as_ref()
    }

    pub fn set_query_dsl(mut self, query_dsl: Option<Value>) -> Self {
        self.query_dsl = query_dsl;
        self
    }

    pub fn into_value(self) -> serde_json::Value {
        serde_json::to_value(&self).expect("failed to convert ZDBQuery to a json Value")
    }
}

#[pg_extern(immutable, parallel_safe)]
fn to_query_dsl(query: ZDBQuery) -> Option<Json> {
    match query.query_dsl() {
        Some(json) => Some(Json(json.clone())),
        None => None,
    }
}

#[pg_extern(immutable, parallel_safe)]
fn to_queries_dsl(queries: Array<ZDBQuery>) -> Vec<Option<Json>> {
    let mut result = Vec::new();
    for query in queries.iter() {
        match query {
            Some(query) => result.push(match query.query_dsl() {
                Some(json) => Some(Json(json.clone())),
                None => None,
            }),
            None => result.push(None),
        }
    }

    result
}

#[cfg(any(test, feature = "pg_test"))]
mod tests {
    use crate::zdbquery::*;
    use serde_json::json;

    #[pg_test]
    fn test_zdbquery_in_with_query_string() {
        let input = std::ffi::CStr::from_bytes_with_nul(b"this is a test\0").unwrap();
        let zdbquery = pg_catalog::zdbquery_in(input);
        let json = serde_json::to_value(&zdbquery).unwrap();

        assert_eq!(
            json,
            json!( {"query_dsl":{"query_string":{"query":"this is a test"}}} )
        );
    }

    #[pg_test]
    fn test_zdbquery_in_with_query_dsl() {
        let input = std::ffi::CStr::from_bytes_with_nul(b" {\"match_all\":{}} \0").unwrap();
        let zdbquery = pg_catalog::zdbquery_in(input);
        let json = serde_json::to_value(&zdbquery).unwrap();

        assert_eq!(json, json!( {"query_dsl":{"match_all":{}}} ));
    }

    #[pg_test]
    fn test_zdbquery_in_with_full_query() {
        let input = std::ffi::CStr::from_bytes_with_nul(
            b" {\"query_dsl\":{\"query_string\":{\"query\":\"this is a test\"}}} \0",
        )
        .unwrap();
        let zdbquery = pg_catalog::zdbquery_in(input);
        let json = serde_json::to_value(&zdbquery).unwrap();

        assert_eq!(
            json,
            json!( {"query_dsl":{"query_string":{"query":"this is a test"}}} )
        );
    }
}