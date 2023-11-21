/*
 * Docker Engine API
 *
 * The Engine API is an HTTP API served by Docker Engine. It is the API the Docker client uses to communicate with the Engine, so everything the Docker client can do can be done with the API.  Most of the client's commands map directly to API endpoints (e.g. `docker ps` is `GET /containers/json`). The notable exception is running containers, which consists of several API calls.  # Errors  The API uses standard HTTP status codes to indicate the success or failure of the API call. The body of the response will be JSON in the following format:  ``` {   \"message\": \"page not found\" } ```  # Versioning  The API is usually changed in each release, so API calls are versioned to ensure that clients don't break. To lock to a specific version of the API, you prefix the URL with its version, for example, call `/v1.30/info` to use the v1.30 version of the `/info` endpoint. If the API version specified in the URL is not supported by the daemon, a HTTP `400 Bad Request` error message is returned.  If you omit the version-prefix, the current version of the API (v1.43) is used. For example, calling `/info` is the same as calling `/v1.43/info`. Using the API without a version-prefix is deprecated and will be removed in a future release.  Engine releases in the near future should support this version of the API, so your client will continue to work even if it is talking to a newer Engine.  The API uses an open schema model, which means server may add extra properties to responses. Likewise, the server will ignore any extra query parameters and request body properties. When you write clients, you need to ignore additional properties in responses to ensure they do not break when talking to newer daemons.   # Authentication  Authentication for registries is handled client side. The client has to send authentication details to various endpoints that need to communicate with registries, such as `POST /images/(name)/push`. These are sent as `X-Registry-Auth` header as a [base64url encoded](https://tools.ietf.org/html/rfc4648#section-5) (JSON) string with the following structure:  ``` {   \"username\": \"string\",   \"password\": \"string\",   \"email\": \"string\",   \"serveraddress\": \"string\" } ```  The `serveraddress` is a domain/IP without a protocol. Throughout this structure, double quotes are required.  If you have already got an identity token from the [`/auth` endpoint](#operation/SystemAuth), you can just pass this instead of credentials:  ``` {   \"identitytoken\": \"9cbaf023786cd7...\" } ```
 *
 * The version of the OpenAPI document: 1.43
 *
 * Generated by: https://openapi-generator.tech
 */

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Swarm {
    /// The ID of the swarm.
    #[serde(rename = "ID", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "Version", skip_serializing_if = "Option::is_none")]
    pub version: Option<Box<crate::models::ObjectVersion>>,
    /// Date and time at which the swarm was initialised in [RFC 3339](https://www.ietf.org/rfc/rfc3339.txt) format with nano-seconds.
    #[serde(rename = "CreatedAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Date and time at which the swarm was last updated in [RFC 3339](https://www.ietf.org/rfc/rfc3339.txt) format with nano-seconds.
    #[serde(rename = "UpdatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(rename = "Spec", skip_serializing_if = "Option::is_none")]
    pub spec: Option<Box<crate::models::SwarmSpec>>,
    #[serde(rename = "TLSInfo", skip_serializing_if = "Option::is_none")]
    pub tls_info: Option<Box<crate::models::TlsInfo>>,
    /// Whether there is currently a root CA rotation in progress for the swarm
    #[serde(
        rename = "RootRotationInProgress",
        skip_serializing_if = "Option::is_none"
    )]
    pub root_rotation_in_progress: Option<bool>,
    /// DataPathPort specifies the data path port number for data traffic. Acceptable port range is 1024 to 49151. If no port is set or is set to 0, the default port (4789) is used.
    #[serde(rename = "DataPathPort", skip_serializing_if = "Option::is_none")]
    pub data_path_port: Option<i32>,
    /// Default Address Pool specifies default subnet pools for global scope networks.
    #[serde(rename = "DefaultAddrPool", skip_serializing_if = "Option::is_none")]
    pub default_addr_pool: Option<Vec<String>>,
    /// SubnetSize specifies the subnet size of the networks created from the default subnet pool.
    #[serde(rename = "SubnetSize", skip_serializing_if = "Option::is_none")]
    pub subnet_size: Option<i32>,
    #[serde(rename = "JoinTokens", skip_serializing_if = "Option::is_none")]
    pub join_tokens: Option<Box<crate::models::JoinTokens>>,
}

impl Swarm {
    pub fn new() -> Swarm {
        Swarm {
            id: None,
            version: None,
            created_at: None,
            updated_at: None,
            spec: None,
            tls_info: None,
            root_rotation_in_progress: None,
            data_path_port: None,
            default_addr_pool: None,
            subnet_size: None,
            join_tokens: None,
        }
    }
}
