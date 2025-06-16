use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    Tcp { host: String, port: u16 },
    Stdio { command: String, args: Vec<String> },
}

impl Target {
    pub fn parse(input: &str) -> Result<Self, String> {
        if input.starts_with("tcp://") {
            let remainder = &input[6..];
            Self::parse_tcp(remainder)
        } else if input.starts_with("cmd://") {
            let remainder = &input[6..];
            Self::parse_stdio(remainder)
        } else {
            // Implicit TCP
            Self::parse_tcp(input)
        }
    }

    fn parse_tcp(input: &str) -> Result<Self, String> {
        if input.is_empty() {
            return Err("Empty host specification".to_string());
        }

        // Handle port-only format (e.g., ":8080")
        // But make sure it's not an IPv6 address starting with ::
        if input.starts_with(':') && !input.starts_with("::") {
            let port_str = &input[1..];
            if port_str.is_empty() {
                return Err("Empty port specification".to_string());
            }
            let port = port_str
                .parse::<u16>()
                .map_err(|_| format!("Invalid port: {}", port_str))?;
            return Ok(Target::Tcp {
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
                    return Err("Port is required for TCP targets".to_string());
                } else if remainder.starts_with(':') {
                    let port_str = &remainder[1..];
                    let port = port_str
                        .parse::<u16>()
                        .map_err(|_| format!("Invalid port: {}", port_str))?;
                    return Ok(Target::Tcp { host, port });
                } else {
                    return Err("Invalid character after IPv6 address".to_string());
                }
            } else {
                return Err("Unclosed IPv6 address bracket".to_string());
            }
        }

        // Handle regular host:port
        if let Some(colon_pos) = input.rfind(':') {
            let host = input[..colon_pos].to_string();
            let port_str = &input[colon_pos + 1..];

            // Check if this might be part of an IPv6 address without brackets
            if host.contains(':') {
                // This is likely an IPv6 address without brackets and no port
                Err("Port is required for TCP targets".to_string())
            } else if port_str.is_empty() {
                Err("Empty port specification".to_string())
            } else {
                let port = port_str
                    .parse::<u16>()
                    .map_err(|_| format!("Invalid port: {}", port_str))?;
                Ok(Target::Tcp { host, port })
            }
        } else {
            Err("Port is required for TCP targets".to_string())
        }
    }

    fn parse_stdio(input: &str) -> Result<Self, String> {
        if input.is_empty() {
            return Err("Empty command specification".to_string());
        }

        // Simple shell-like parsing
        let parts =
            shell_words::split(input).map_err(|e| format!("Failed to parse command: {}", e))?;

        if parts.is_empty() {
            return Err("Empty command after parsing".to_string());
        }

        let command = parts[0].clone();
        let args = parts[1..].to_vec();

        Ok(Target::Stdio { command, args })
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Target::Tcp { host, port } => {
                // Check if host is an IPv6 address (contains colons but not already bracketed)
                if host.contains(':') && !host.starts_with('[') {
                    write!(f, "tcp://[{}]:{}", host, port)
                } else {
                    write!(f, "tcp://{}:{}", host, port)
                }
            }
            Target::Stdio { command, args } => {
                if args.is_empty() {
                    write!(f, "cmd://{}", command)
                } else {
                    write!(f, "cmd://{} {}", command, shell_words::join(args))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_parsing() {
        struct TestCase {
            input: &'static str,
            expected: Result<Target, &'static str>,
            description: &'static str,
        }

        let test_cases = vec![
            // Implicit TCP
            TestCase {
                input: "example.com",
                expected: Err("Port is required for TCP targets"),
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
                expected: Err("Port is required for TCP targets"),
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
                expected: Err("Port is required for TCP targets"),
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
                expected: Err("Port is required for TCP targets"),
                description: "IPv6 without brackets (no port)",
            },
            TestCase {
                input: "2001:db8::1",
                expected: Err("Port is required for TCP targets"),
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
                expected: Err("Empty host specification"),
                description: "empty input",
            },
            TestCase {
                input: "tcp://",
                expected: Err("Empty host specification"),
                description: "TCP scheme without host",
            },
            TestCase {
                input: "cmd://",
                expected: Err("Empty command specification"),
                description: "stdio scheme without command",
            },
            TestCase {
                input: "example.com:",
                expected: Err("Empty port specification"),
                description: "host with colon but no port",
            },
            TestCase {
                input: "example.com:abc",
                expected: Err("Invalid port: abc"),
                description: "invalid port (not a number)",
            },
            TestCase {
                input: "example.com:99999",
                expected: Err("Invalid port: 99999"),
                description: "port out of range",
            },
            TestCase {
                input: "[::1",
                expected: Err("Unclosed IPv6 address bracket"),
                description: "unclosed IPv6 bracket",
            },
            TestCase {
                input: "[::1]x",
                expected: Err("Invalid character after IPv6 address"),
                description: "invalid character after IPv6",
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
                (Err(expected_msg), Err(actual_err)) => {
                    assert_eq!(
                        *expected_msg, &actual_err,
                        "Failed for '{}': {}",
                        test_case.input, test_case.description
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
