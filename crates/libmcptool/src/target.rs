use std::fmt;

use crate::{Error, Result, auth::validate_auth_name};

/// Represents a connection target for MCP servers.
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    /// TCP connection target.
    Tcp {
        /// Hostname or IP address.
        host: String,
        /// Port number.
        port: u16,
    },
    /// Stdio connection target (subprocess).
    Stdio {
        /// Command to execute.
        command: String,
        /// Arguments for the command.
        args: Vec<String>,
    },
    /// HTTP connection target.
    Http {
        /// Hostname or IP address.
        host: String,
        /// Port number.
        port: u16,
        /// Optional URL path.
        path: Option<String>,
    },
    /// HTTPS connection target.
    Https {
        /// Hostname or IP address.
        host: String,
        /// Port number.
        port: u16,
        /// Optional URL path.
        path: Option<String>,
    },
    /// Auth target (references a stored authentication entry).
    Auth {
        /// Name of the stored auth entry.
        name: String,
    },
}

impl Target {
    pub fn parse(input: &str) -> Result<Self> {
        if let Some(remainder) = input.strip_prefix("tcp://") {
            Self::parse_tcp(remainder)
        } else if let Some(remainder) = input.strip_prefix("cmd://") {
            Self::parse_stdio(remainder)
        } else if let Some(remainder) = input.strip_prefix("https://") {
            Self::parse_https(remainder)
        } else if let Some(remainder) = input.strip_prefix("http://") {
            Self::parse_http(remainder)
        } else if let Some(remainder) = input.strip_prefix("auth://") {
            Self::parse_auth(remainder)
        } else {
            // Implicit TCP
            Self::parse_tcp(input)
        }
    }

    /// Parses a TCP target specification from the given input string.
    fn parse_tcp(input: &str) -> Result<Self> {
        if input.is_empty() {
            return Err(Error::Format("Empty host specification".to_string()));
        }

        // Handle port-only format (e.g., ":8080")
        // But make sure it's not an IPv6 address starting with ::
        if input.starts_with(':') && !input.starts_with("::") {
            let port_str = &input[1..];
            if port_str.is_empty() {
                return Err(Error::Format("Empty port specification".to_string()));
            }
            let port = port_str
                .parse::<u16>()
                .map_err(|_| Error::Format(format!("Invalid port: {port_str}")))?;
            return Ok(Self::Tcp {
                host: "0.0.0.0".to_string(),
                port,
            });
        }

        // Handle IPv6 addresses in brackets
        if input.starts_with('[') {
            if let Some(end) = input.find(']') {
                let host = input[1..end].to_string();
                let remainder = &input[end + 1..];

                if remainder.is_empty() {
                    return Err(Error::Format(
                        "Port is required for TCP targets".to_string(),
                    ));
                } else if let Some(port_str) = remainder.strip_prefix(':') {
                    let port = port_str
                        .parse::<u16>()
                        .map_err(|_| Error::Format(format!("Invalid port: {port_str}")))?;
                    return Ok(Self::Tcp { host, port });
                } else {
                    return Err(Error::Format(
                        "Invalid character after IPv6 address".to_string(),
                    ));
                }
            } else {
                return Err(Error::Format("Unclosed IPv6 address bracket".to_string()));
            }
        }

        // Handle regular host:port
        if let Some(colon_pos) = input.rfind(':') {
            let host = input[..colon_pos].to_string();
            let port_str = &input[colon_pos + 1..];

            // Check if this might be part of an IPv6 address without brackets
            if host.contains(':') {
                // This is likely an IPv6 address without brackets and no port
                Err(Error::Format(
                    "Port is required for TCP targets".to_string(),
                ))
            } else if port_str.is_empty() {
                Err(Error::Format("Empty port specification".to_string()))
            } else {
                let port = port_str
                    .parse::<u16>()
                    .map_err(|_| Error::Format(format!("Invalid port: {port_str}")))?;
                Ok(Self::Tcp { host, port })
            }
        } else {
            Err(Error::Format(
                "Port is required for TCP targets".to_string(),
            ))
        }
    }

    /// Parses a stdio target specification from the given input string.
    fn parse_stdio(input: &str) -> Result<Self> {
        if input.is_empty() {
            return Err(Error::Format("Empty command specification".to_string()));
        }

        // Simple shell-like parsing
        let parts = shell_words::split(input)
            .map_err(|e| Error::Format(format!("Failed to parse command: {e}")))?;

        if parts.is_empty() {
            return Err(Error::Format("Empty command after parsing".to_string()));
        }

        let command = parts[0].clone();
        let args = parts[1..].to_vec();

        Ok(Self::Stdio { command, args })
    }

    /// Parses an HTTP target specification from the given input string.
    fn parse_http(input: &str) -> Result<Self> {
        Self::parse_http_common(input, 80, |host, port, path| Self::Http {
            host,
            port,
            path,
        })
    }

    /// Parses an HTTPS target specification from the given input string.
    fn parse_https(input: &str) -> Result<Self> {
        Self::parse_http_common(input, 443, |host, port, path| Self::Https {
            host,
            port,
            path,
        })
    }

    /// Common parsing logic for HTTP and HTTPS targets.
    fn parse_http_common<F>(input: &str, default_port: u16, constructor: F) -> Result<Self>
    where
        F: Fn(String, u16, Option<String>) -> Self,
    {
        if input.is_empty() {
            return Err(Error::Format("Empty host specification".to_string()));
        }

        // Split off the path component first (everything after first / that isn't part of IPv6)
        let (host_port_part, path) = if input.starts_with('[') {
            // IPv6: find the closing bracket first
            if let Some(bracket_end) = input.find(']') {
                let after_bracket = &input[bracket_end + 1..];
                if let Some(slash_pos) = after_bracket.find('/') {
                    let path_start = bracket_end + 1 + slash_pos;
                    let path = &input[path_start..];
                    let path = if path == "/" {
                        None
                    } else {
                        Some(path.to_string())
                    };
                    (&input[..path_start], path)
                } else {
                    (input, None)
                }
            } else {
                (input, None)
            }
        } else if let Some(slash_pos) = input.find('/') {
            let path = &input[slash_pos..];
            let path = if path == "/" {
                None
            } else {
                Some(path.to_string())
            };
            (&input[..slash_pos], path)
        } else {
            (input, None)
        };

        // Handle IPv6 addresses in brackets
        if host_port_part.starts_with('[') {
            if let Some(end) = host_port_part.find(']') {
                let host = host_port_part[1..end].to_string();
                let remainder = &host_port_part[end + 1..];

                if remainder.is_empty() {
                    return Ok(constructor(host, default_port, path));
                } else if let Some(port_str) = remainder.strip_prefix(':') {
                    let port = port_str
                        .parse::<u16>()
                        .map_err(|_| Error::Format(format!("Invalid port: {port_str}")))?;
                    return Ok(constructor(host, port, path));
                } else {
                    return Err(Error::Format(
                        "Invalid character after IPv6 address".to_string(),
                    ));
                }
            } else {
                return Err(Error::Format("Unclosed IPv6 address bracket".to_string()));
            }
        }

        // Handle regular host or host:port
        if let Some(colon_pos) = host_port_part.rfind(':') {
            let host = host_port_part[..colon_pos].to_string();
            let port_str = &host_port_part[colon_pos + 1..];

            // Check if this might be part of an IPv6 address without brackets
            if host.contains(':') {
                // This is likely an IPv6 address without brackets and no port
                Ok(constructor(host_port_part.to_string(), default_port, path))
            } else if port_str.is_empty() {
                Err(Error::Format("Empty port specification".to_string()))
            } else {
                let port = port_str
                    .parse::<u16>()
                    .map_err(|_| Error::Format(format!("Invalid port: {port_str}")))?;
                Ok(constructor(host, port, path))
            }
        } else {
            // Just a hostname, use default port
            Ok(constructor(host_port_part.to_string(), default_port, path))
        }
    }

    /// Parses an auth target specification from the given input string.
    fn parse_auth(input: &str) -> Result<Self> {
        if input.is_empty() {
            return Err(Error::Format("Empty auth name".to_string()));
        }

        // Validate the auth name using the shared validation function
        validate_auth_name(input)?;

        Ok(Self::Auth {
            name: input.to_string(),
        })
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tcp { host, port } => {
                // Check if host is an IPv6 address (contains colons but not already bracketed)
                if host.contains(':') && !host.starts_with('[') {
                    write!(f, "tcp://[{host}]:{port}")
                } else {
                    write!(f, "tcp://{host}:{port}")
                }
            }
            Self::Stdio { command, args } => {
                if args.is_empty() {
                    write!(f, "cmd://{command}")
                } else {
                    write!(f, "cmd://{} {}", command, shell_words::join(args))
                }
            }
            Self::Http { host, port, path } => {
                let path_str = path.as_deref().unwrap_or("");
                // Check if host is an IPv6 address
                if host.contains(':') && !host.starts_with('[') {
                    if *port == 80 {
                        write!(f, "http://[{host}]{path_str}")
                    } else {
                        write!(f, "http://[{host}]:{port}{path_str}")
                    }
                } else if *port == 80 {
                    write!(f, "http://{host}{path_str}")
                } else {
                    write!(f, "http://{host}:{port}{path_str}")
                }
            }
            Self::Https { host, port, path } => {
                let path_str = path.as_deref().unwrap_or("");
                // Check if host is an IPv6 address
                if host.contains(':') && !host.starts_with('[') {
                    if *port == 443 {
                        write!(f, "https://[{host}]{path_str}")
                    } else {
                        write!(f, "https://[{host}]:{port}{path_str}")
                    }
                } else if *port == 443 {
                    write!(f, "https://{host}{path_str}")
                } else {
                    write!(f, "https://{host}:{port}{path_str}")
                }
            }
            Self::Auth { name } => {
                write!(f, "auth://{name}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format_err(msg: &str) -> Error {
        Error::Format(msg.to_string())
    }

    #[test]
    fn test_target_parsing() {
        struct TestCase {
            input: &'static str,
            expected: Result<Target>,
            description: &'static str,
        }

        let test_cases = vec![
            // Implicit TCP
            TestCase {
                input: "example.com",
                expected: Err(format_err("Port is required for TCP targets")),
                description: "implicit TCP without port",
            },
            TestCase {
                input: "example.com:8080",
                expected: Ok(Target::Tcp {
                    host: "example.com".to_string(),
                    port: 8080,
                }),
                description: "implicit TCP with port",
            },
            TestCase {
                input: "localhost:3000",
                expected: Ok(Target::Tcp {
                    host: "localhost".to_string(),
                    port: 3000,
                }),
                description: "localhost with port",
            },
            // Port-only format
            TestCase {
                input: ":8080",
                expected: Ok(Target::Tcp {
                    host: "0.0.0.0".to_string(),
                    port: 8080,
                }),
                description: "port-only format",
            },
            TestCase {
                input: "tcp://:3000",
                expected: Ok(Target::Tcp {
                    host: "0.0.0.0".to_string(),
                    port: 3000,
                }),
                description: "explicit TCP with port-only",
            },
            // Explicit TCP
            TestCase {
                input: "tcp://example.com",
                expected: Err(format_err("Port is required for TCP targets")),
                description: "explicit TCP without port",
            },
            TestCase {
                input: "tcp://example.com:9999",
                expected: Ok(Target::Tcp {
                    host: "example.com".to_string(),
                    port: 9999,
                }),
                description: "explicit TCP with port",
            },
            // IPv6
            TestCase {
                input: "[::1]",
                expected: Err(format_err("Port is required for TCP targets")),
                description: "IPv6 localhost without port",
            },
            TestCase {
                input: "[::1]:8080",
                expected: Ok(Target::Tcp {
                    host: "::1".to_string(),
                    port: 8080,
                }),
                description: "IPv6 localhost with port",
            },
            TestCase {
                input: "tcp://[2001:db8::1]:443",
                expected: Ok(Target::Tcp {
                    host: "2001:db8::1".to_string(),
                    port: 443,
                }),
                description: "explicit TCP with IPv6 and port",
            },
            TestCase {
                input: "::1",
                expected: Err(format_err("Port is required for TCP targets")),
                description: "IPv6 without brackets (no port)",
            },
            TestCase {
                input: "2001:db8::1",
                expected: Err(format_err("Port is required for TCP targets")),
                description: "IPv6 address without brackets",
            },
            // Stdio
            TestCase {
                input: "cmd://mcp-server",
                expected: Ok(Target::Stdio {
                    command: "mcp-server".to_string(),
                    args: vec![],
                }),
                description: "stdio command without args",
            },
            TestCase {
                input: "cmd://./my-server --port 8080 --verbose",
                expected: Ok(Target::Stdio {
                    command: "./my-server".to_string(),
                    args: vec![
                        "--port".to_string(),
                        "8080".to_string(),
                        "--verbose".to_string(),
                    ],
                }),
                description: "stdio command with args",
            },
            TestCase {
                input: r#"cmd://server --name "My Server" --path "/some path/""#,
                expected: Ok(Target::Stdio {
                    command: "server".to_string(),
                    args: vec![
                        "--name".to_string(),
                        "My Server".to_string(),
                        "--path".to_string(),
                        "/some path/".to_string(),
                    ],
                }),
                description: "stdio command with quoted args",
            },
            // Error cases
            TestCase {
                input: "",
                expected: Err(format_err("Empty host specification")),
                description: "empty input",
            },
            TestCase {
                input: "tcp://",
                expected: Err(format_err("Empty host specification")),
                description: "TCP scheme without host",
            },
            TestCase {
                input: "cmd://",
                expected: Err(format_err("Empty command specification")),
                description: "stdio scheme without command",
            },
            TestCase {
                input: "example.com:",
                expected: Err(format_err("Empty port specification")),
                description: "host with colon but no port",
            },
            TestCase {
                input: "example.com:abc",
                expected: Err(format_err("Invalid port: abc")),
                description: "invalid port (not a number)",
            },
            TestCase {
                input: "example.com:99999",
                expected: Err(format_err("Invalid port: 99999")),
                description: "port out of range",
            },
            TestCase {
                input: "[::1",
                expected: Err(format_err("Unclosed IPv6 address bracket")),
                description: "unclosed IPv6 bracket",
            },
            TestCase {
                input: "[::1]x",
                expected: Err(format_err("Invalid character after IPv6 address")),
                description: "invalid character after IPv6",
            },
            // HTTP tests
            TestCase {
                input: "http://example.com",
                expected: Ok(Target::Http {
                    host: "example.com".to_string(),
                    port: 80,
                    path: None,
                }),
                description: "HTTP with default port",
            },
            TestCase {
                input: "http://example.com:8080",
                expected: Ok(Target::Http {
                    host: "example.com".to_string(),
                    port: 8080,
                    path: None,
                }),
                description: "HTTP with custom port",
            },
            TestCase {
                input: "http://example.com/api/v1",
                expected: Ok(Target::Http {
                    host: "example.com".to_string(),
                    port: 80,
                    path: Some("/api/v1".to_string()),
                }),
                description: "HTTP with path",
            },
            TestCase {
                input: "http://example.com:8080/api/v1",
                expected: Ok(Target::Http {
                    host: "example.com".to_string(),
                    port: 8080,
                    path: Some("/api/v1".to_string()),
                }),
                description: "HTTP with port and path",
            },
            TestCase {
                input: "http://[::1]",
                expected: Ok(Target::Http {
                    host: "::1".to_string(),
                    port: 80,
                    path: None,
                }),
                description: "HTTP with IPv6 default port",
            },
            TestCase {
                input: "http://[2001:db8::1]:8080",
                expected: Ok(Target::Http {
                    host: "2001:db8::1".to_string(),
                    port: 8080,
                    path: None,
                }),
                description: "HTTP with IPv6 and custom port",
            },
            TestCase {
                input: "http://[::1]/api",
                expected: Ok(Target::Http {
                    host: "::1".to_string(),
                    port: 80,
                    path: Some("/api".to_string()),
                }),
                description: "HTTP with IPv6 and path",
            },
            TestCase {
                input: "http://::1",
                expected: Ok(Target::Http {
                    host: "::1".to_string(),
                    port: 80,
                    path: None,
                }),
                description: "HTTP with IPv6 no brackets",
            },
            // HTTPS tests
            TestCase {
                input: "https://example.com",
                expected: Ok(Target::Https {
                    host: "example.com".to_string(),
                    port: 443,
                    path: None,
                }),
                description: "HTTPS with default port",
            },
            TestCase {
                input: "https://example.com:8443",
                expected: Ok(Target::Https {
                    host: "example.com".to_string(),
                    port: 8443,
                    path: None,
                }),
                description: "HTTPS with custom port",
            },
            TestCase {
                input: "https://mcp.linear.app/mcp",
                expected: Ok(Target::Https {
                    host: "mcp.linear.app".to_string(),
                    port: 443,
                    path: Some("/mcp".to_string()),
                }),
                description: "HTTPS with path (linear example)",
            },
            TestCase {
                input: "https://[::1]",
                expected: Ok(Target::Https {
                    host: "::1".to_string(),
                    port: 443,
                    path: None,
                }),
                description: "HTTPS with IPv6 default port",
            },
            TestCase {
                input: "https://[2001:db8::1]:8443",
                expected: Ok(Target::Https {
                    host: "2001:db8::1".to_string(),
                    port: 8443,
                    path: None,
                }),
                description: "HTTPS with IPv6 and custom port",
            },
            // Auth tests
            TestCase {
                input: "auth://myservice",
                expected: Ok(Target::Auth {
                    name: "myservice".to_string(),
                }),
                description: "Auth with simple name",
            },
            TestCase {
                input: "auth://my_oauth_service",
                expected: Ok(Target::Auth {
                    name: "my_oauth_service".to_string(),
                }),
                description: "Auth with underscored name",
            },
            TestCase {
                input: "auth://MyAuth123",
                expected: Ok(Target::Auth {
                    name: "MyAuth123".to_string(),
                }),
                description: "Auth with mixed case and numbers",
            },
            TestCase {
                input: "auth://my-oauth-service",
                expected: Err(format_err(
                    "Authentication name 'my-oauth-service' is invalid. Names can only contain letters, numbers, and underscores (a-zA-Z0-9_)",
                )),
                description: "Auth with hyphenated name (invalid)",
            },
            TestCase {
                input: "auth://my:service",
                expected: Err(format_err(
                    "Authentication name 'my:service' is invalid. Names can only contain letters, numbers, and underscores (a-zA-Z0-9_)",
                )),
                description: "Auth with colon (invalid)",
            },
            TestCase {
                input: "auth://my/service",
                expected: Err(format_err(
                    "Authentication name 'my/service' is invalid. Names can only contain letters, numbers, and underscores (a-zA-Z0-9_)",
                )),
                description: "Auth with slash (invalid)",
            },
            TestCase {
                input: "auth://",
                expected: Err(format_err("Empty auth name")),
                description: "Auth scheme without name",
            },
            // HTTP/HTTPS error cases
            TestCase {
                input: "http://",
                expected: Err(format_err("Empty host specification")),
                description: "HTTP scheme without host",
            },
            TestCase {
                input: "https://",
                expected: Err(format_err("Empty host specification")),
                description: "HTTPS scheme without host",
            },
            TestCase {
                input: "http://example.com:",
                expected: Err(format_err("Empty port specification")),
                description: "HTTP with colon but no port",
            },
            TestCase {
                input: "https://example.com:abc",
                expected: Err(format_err("Invalid port: abc")),
                description: "HTTPS invalid port",
            },
            TestCase {
                input: "http://[::1",
                expected: Err(format_err("Unclosed IPv6 address bracket")),
                description: "HTTP unclosed IPv6 bracket",
            },
        ];

        for test_case in test_cases {
            match (&test_case.expected, Target::parse(test_case.input)) {
                (Ok(expected), Ok(actual)) => {
                    assert_eq!(
                        expected, &actual,
                        "Failed for '{}': {}",
                        test_case.input, test_case.description
                    );
                }
                (Err(expected_err), Err(actual_err)) => {
                    assert_eq!(
                        expected_err.to_string(),
                        actual_err.to_string(),
                        "Failed for '{}': {}",
                        test_case.input,
                        test_case.description
                    );
                }
                (Ok(_), Err(e)) => {
                    panic!(
                        "Expected success for '{}' ({}), but got error: {}",
                        test_case.input, test_case.description, e
                    );
                }
                (Err(_), Ok(t)) => {
                    panic!(
                        "Expected error for '{}' ({}), but got success: {:?}",
                        test_case.input, test_case.description, t
                    );
                }
            }
        }
    }

    #[test]
    fn test_target_display() {
        struct TestCase {
            target: Target,
            expected: &'static str,
            description: &'static str,
        }

        let test_cases = vec![
            TestCase {
                target: Target::Tcp {
                    host: "example.com".to_string(),
                    port: 8080,
                },
                expected: "tcp://example.com:8080",
                description: "TCP with port",
            },
            TestCase {
                target: Target::Tcp {
                    host: "::1".to_string(),
                    port: 3000,
                },
                expected: "tcp://[::1]:3000",
                description: "IPv6 with port",
            },
            TestCase {
                target: Target::Stdio {
                    command: "./server".to_string(),
                    args: vec![],
                },
                expected: "cmd://./server",
                description: "stdio without args",
            },
            TestCase {
                target: Target::Stdio {
                    command: "./server".to_string(),
                    args: vec!["--verbose".to_string()],
                },
                expected: "cmd://./server --verbose",
                description: "stdio with args",
            },
            TestCase {
                target: Target::Stdio {
                    command: "server".to_string(),
                    args: vec!["--path".to_string(), "/some path/".to_string()],
                },
                expected: r#"cmd://server --path '/some path/'"#,
                description: "stdio with quoted args",
            },
            // HTTP display tests
            TestCase {
                target: Target::Http {
                    host: "example.com".to_string(),
                    port: 80,
                    path: None,
                },
                expected: "http://example.com",
                description: "HTTP with default port",
            },
            TestCase {
                target: Target::Http {
                    host: "example.com".to_string(),
                    port: 8080,
                    path: None,
                },
                expected: "http://example.com:8080",
                description: "HTTP with custom port",
            },
            TestCase {
                target: Target::Http {
                    host: "example.com".to_string(),
                    port: 80,
                    path: Some("/api/v1".to_string()),
                },
                expected: "http://example.com/api/v1",
                description: "HTTP with path",
            },
            TestCase {
                target: Target::Http {
                    host: "example.com".to_string(),
                    port: 8080,
                    path: Some("/api".to_string()),
                },
                expected: "http://example.com:8080/api",
                description: "HTTP with port and path",
            },
            TestCase {
                target: Target::Http {
                    host: "::1".to_string(),
                    port: 80,
                    path: None,
                },
                expected: "http://[::1]",
                description: "HTTP IPv6 with default port",
            },
            TestCase {
                target: Target::Http {
                    host: "2001:db8::1".to_string(),
                    port: 8080,
                    path: None,
                },
                expected: "http://[2001:db8::1]:8080",
                description: "HTTP IPv6 with custom port",
            },
            // HTTPS display tests
            TestCase {
                target: Target::Https {
                    host: "example.com".to_string(),
                    port: 443,
                    path: None,
                },
                expected: "https://example.com",
                description: "HTTPS with default port",
            },
            TestCase {
                target: Target::Https {
                    host: "example.com".to_string(),
                    port: 8443,
                    path: None,
                },
                expected: "https://example.com:8443",
                description: "HTTPS with custom port",
            },
            TestCase {
                target: Target::Https {
                    host: "mcp.linear.app".to_string(),
                    port: 443,
                    path: Some("/mcp".to_string()),
                },
                expected: "https://mcp.linear.app/mcp",
                description: "HTTPS with path (linear example)",
            },
            TestCase {
                target: Target::Https {
                    host: "::1".to_string(),
                    port: 443,
                    path: None,
                },
                expected: "https://[::1]",
                description: "HTTPS IPv6 with default port",
            },
            TestCase {
                target: Target::Https {
                    host: "2001:db8::1".to_string(),
                    port: 8443,
                    path: None,
                },
                expected: "https://[2001:db8::1]:8443",
                description: "HTTPS IPv6 with custom port",
            },
            // Auth display tests
            TestCase {
                target: Target::Auth {
                    name: "myservice".to_string(),
                },
                expected: "auth://myservice",
                description: "Auth with simple name",
            },
            TestCase {
                target: Target::Auth {
                    name: "my_oauth_service".to_string(),
                },
                expected: "auth://my_oauth_service",
                description: "Auth with underscored name",
            },
        ];

        for test_case in test_cases {
            assert_eq!(
                test_case.target.to_string(),
                test_case.expected,
                "Failed display for: {}",
                test_case.description
            );
        }
    }
}
