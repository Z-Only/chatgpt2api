use std::collections::BTreeMap;

use reqwest::header::HeaderMap;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UsageLimit {
    pub name: String,
    pub limit: Option<u64>,
    pub remaining: Option<u64>,
    pub reset_seconds: Option<u64>,
}

pub fn parse_usage_limit_headers(headers: &HeaderMap) -> Vec<UsageLimit> {
    let mut limits = BTreeMap::<String, UsageLimit>::new();

    for name in ["requests", "tokens"] {
        let mut usage = UsageLimit {
            name: name.to_string(),
            ..UsageLimit::default()
        };
        usage.limit = parse_u64(headers, &format!("x-ratelimit-limit-{name}"));
        usage.remaining = parse_u64(headers, &format!("x-ratelimit-remaining-{name}"));
        usage.reset_seconds = parse_u64(headers, &format!("x-ratelimit-reset-{name}"));

        if usage.limit.is_some() || usage.remaining.is_some() || usage.reset_seconds.is_some() {
            limits.insert(name.to_string(), usage);
        }
    }

    limits.into_values().collect()
}

fn parse_u64(headers: &HeaderMap, name: &str) -> Option<u64> {
    headers.get(name)?.to_str().ok()?.parse().ok()
}
