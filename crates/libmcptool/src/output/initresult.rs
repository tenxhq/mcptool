use crate::output::Output;
use crate::Result;

/// Display the InitializeResult in either JSON or formatted text
pub fn init_result(
    output: &Output,
    init_result: &tenx_mcp::schema::InitializeResult,
) -> Result<()> {
    if output.json {
        // Output as JSON
        output.json_value(init_result)?;
    } else {
        // Output as formatted text
        output.h1("Server Information")?;
        output.text("")?;

        let out = output.indent();

        // Protocol version
        out.kv("Protocol Version", &init_result.protocol_version)?;

        // Server info
        out.text("")?;
        out.h2("Server")?;
        let out = out.indent();
        out.kv("Name", &init_result.server_info.name)?;
        out.kv("Version", &init_result.server_info.version)?;
        if let Some(title) = &init_result.server_info.title {
            out.kv("Title", title)?;
        }

        // Server capabilities
        out.text("")?;
        out.h2("Capabilities")?;
        let out = out.indent();

        // Tools
        if let Some(tools) = &init_result.capabilities.tools {
            if tools.list_changed.unwrap_or(false) {
                out.kv("Tools", "Supported (dynamic list)")?;
            } else {
                out.kv("Tools", "Supported")?;
            }
        } else {
            out.kv("Tools", "Not supported")?;
        }

        // Resources
        if let Some(resources) = &init_result.capabilities.resources {
            if resources.list_changed.unwrap_or(false) {
                out.kv("Resources", "Supported (dynamic list)")?;
            } else {
                out.kv("Resources", "Supported")?;
            }

            if resources.subscribe.unwrap_or(false) {
                let out = out.indent();
                out.text("• Subscriptions supported")?;
            }
        } else {
            out.kv("Resources", "Not supported")?;
        }

        // Prompts
        if let Some(prompts) = &init_result.capabilities.prompts {
            if prompts.list_changed.unwrap_or(false) {
                out.kv("Prompts", "Supported (dynamic list)")?;
            } else {
                out.kv("Prompts", "Supported")?;
            }
        } else {
            out.kv("Prompts", "Not supported")?;
        }

        // Logging
        if let Some(_logging) = &init_result.capabilities.logging {
            out.kv("Logging", "Supported")?;
        } else {
            out.kv("Logging", "Not supported")?;
        }

        // Completions
        if let Some(_completions) = &init_result.capabilities.completions {
            out.kv("Completions", "Supported")?;
        } else {
            out.kv("Completions", "Not supported")?;
        }

        // Experimental capabilities
        if let Some(experimental) = &init_result.capabilities.experimental {
            if !experimental.is_empty() {
                out.text("")?;
                out.h3("Experimental Features")?;
                let out = out.indent();
                for (key, value) in experimental {
                    // Format the value as a pretty JSON string
                    let value_str =
                        serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
                    out.kv(key, &value_str)?;
                }
            }
        }

        // Instructions (if present)
        if let Some(instructions) = &init_result.instructions {
            out.text("")?;
            out.h2("Instructions")?;
            let out = out.indent();
            for line in instructions.lines() {
                out.text(line)?;
            }
        }

        output.text("")?; // Extra blank line at the end
    }
    Ok(())
}
