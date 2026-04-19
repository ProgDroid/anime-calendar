//! Centralised metric name + label-key constants. Change these only
//! when you're prepared to update dashboards and alert rules in lockstep.

pub const CACHE_HITS_TOTAL: &str = "cache_hits_total";
pub const CACHE_MISSES_TOTAL: &str = "cache_misses_total";
pub const CACHE_OPERATION_DURATION_SECONDS: &str = "cache_operation_duration_seconds";

pub const HTTP_REQUESTS_TOTAL: &str = "http_requests_total";
pub const HTTP_REQUEST_DURATION_SECONDS: &str = "http_request_duration_seconds";

pub const DB_QUERIES_TOTAL: &str = "db_queries_total";
pub const DB_QUERY_DURATION_SECONDS: &str = "db_query_duration_seconds";

pub const AUTH_LOGIN_ATTEMPTS_TOTAL: &str = "auth_login_attempts_total";
pub const AUTH_REGISTRATIONS_TOTAL: &str = "auth_registrations_total";
pub const AUTH_OAUTH_ATTEMPTS_TOTAL: &str = "auth_oauth_attempts_total";
pub const EMAIL_VERIFICATIONS_SENT_TOTAL: &str = "email_verifications_sent_total";
pub const EMAIL_VERIFICATIONS_CONFIRMED_TOTAL: &str = "email_verifications_confirmed_total";
pub const PASSWORD_RESETS_REQUESTED_TOTAL: &str = "password_resets_requested_total";
pub const PASSWORD_RESETS_COMPLETED_TOTAL: &str = "password_resets_completed_total";

// Label keys
pub const LABEL_OP: &str = "op";
pub const LABEL_OUTCOME: &str = "outcome";
pub const LABEL_METHOD: &str = "method";
pub const LABEL_PATH: &str = "path";
pub const LABEL_STATUS: &str = "status";
pub const LABEL_PROVIDER: &str = "provider";

// Label values
pub const OUTCOME_OK: &str = "ok";
pub const OUTCOME_ERR: &str = "err";
pub const OUTCOME_FAILED: &str = "failed";
pub const OUTCOME_RATE_LIMITED: &str = "rate_limited";
