/*
 * Docker Engine API
 *
 * The Engine API is an HTTP API served by Docker Engine. It is the API the Docker client uses to communicate with the Engine, so everything the Docker client can do can be done with the API.  Most of the client's commands map directly to API endpoints (e.g. `docker ps` is `GET /containers/json`). The notable exception is running containers, which consists of several API calls.  # Errors  The API uses standard HTTP status codes to indicate the success or failure of the API call. The body of the response will be JSON in the following format:  ``` {   \"message\": \"page not found\" } ```  # Versioning  The API is usually changed in each release, so API calls are versioned to ensure that clients don't break. To lock to a specific version of the API, you prefix the URL with its version, for example, call `/v1.30/info` to use the v1.30 version of the `/info` endpoint. If the API version specified in the URL is not supported by the daemon, a HTTP `400 Bad Request` error message is returned.  If you omit the version-prefix, the current version of the API (v1.43) is used. For example, calling `/info` is the same as calling `/v1.43/info`. Using the API without a version-prefix is deprecated and will be removed in a future release.  Engine releases in the near future should support this version of the API, so your client will continue to work even if it is talking to a newer Engine.  The API uses an open schema model, which means server may add extra properties to responses. Likewise, the server will ignore any extra query parameters and request body properties. When you write clients, you need to ignore additional properties in responses to ensure they do not break when talking to newer daemons.   # Authentication  Authentication for registries is handled client side. The client has to send authentication details to various endpoints that need to communicate with registries, such as `POST /images/(name)/push`. These are sent as `X-Registry-Auth` header as a [base64url encoded](https://tools.ietf.org/html/rfc4648#section-5) (JSON) string with the following structure:  ``` {   \"username\": \"string\",   \"password\": \"string\",   \"email\": \"string\",   \"serveraddress\": \"string\" } ```  The `serveraddress` is a domain/IP without a protocol. Throughout this structure, double quotes are required.  If you have already got an identity token from the [`/auth` endpoint](#operation/SystemAuth), you can just pass this instead of credentials:  ``` {   \"identitytoken\": \"9cbaf023786cd7...\" } ```
 *
 * The version of the OpenAPI document: 1.43
 *
 * Generated by: https://openapi-generator.tech
 */

/// TaskSpecRestartPolicy : Specification for the restart policy which applies to containers created as part of this service.

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaskSpecRestartPolicy {
    /// Condition for restart.
    #[serde(rename = "Condition", skip_serializing_if = "Option::is_none")]
    pub condition: Option<Condition>,
    /// Delay between restart attempts.
    #[serde(rename = "Delay", skip_serializing_if = "Option::is_none")]
    pub delay: Option<i64>,
    /// Maximum attempts to restart a given container before giving up (default value is 0, which is ignored).
    #[serde(rename = "MaxAttempts", skip_serializing_if = "Option::is_none")]
    pub max_attempts: Option<i64>,
    /// Windows is the time window used to evaluate the restart policy (default value is 0, which is unbounded).
    #[serde(rename = "Window", skip_serializing_if = "Option::is_none")]
    pub window: Option<i64>,
}

impl TaskSpecRestartPolicy {
    /// Specification for the restart policy which applies to containers created as part of this service.
    pub fn new() -> TaskSpecRestartPolicy {
        TaskSpecRestartPolicy {
            condition: None,
            delay: None,
            max_attempts: None,
            window: None,
        }
    }
}

/// Condition for restart.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Condition {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "on-failure")]
    OnFailure,
    #[serde(rename = "any")]
    Any,
}

impl Default for Condition {
    fn default() -> Condition {
        Self::None
    }
}
