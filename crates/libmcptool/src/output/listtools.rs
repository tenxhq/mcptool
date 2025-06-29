use crate::output::Output;
use crate::Result;

/// Display the list of tools in either JSON or formatted text
pub fn list_tools_result(
    output: &Output,
    tools_result: &tenx_mcp::schema::ListToolsResult,
) -> Result<()> {
    if output.json {
        // Output as JSON
        output.json_value(tools_result)?;
    } else {
        // Output as formatted text
        if tools_result.tools.is_empty() {
            output.text("No tools.")?;
        } else {
            for tool in &tools_result.tools {
                output.h1(&tool.name)?;
                output.text("")?; // Extra blank line between tools

                let out = output.indent();

                // Description
                if let Some(description) = &tool.description {
                    for line in description.lines() {
                        out.text(line)?;
                    }
                }
                out.text("")?;

                // Annotations
                if let Some(annotations) = &tool.annotations {
                    let out = out.indent();
                    out.h2("Annotations")?;
                    let out = out.indent();
                    if let Some(title) = &annotations.title {
                        out.kv("title", title)?;
                    }
                }

                // Input arguments
                if let Some(properties) = &tool.input_schema.properties {
                    if !properties.is_empty() {
                        let out = out.indent();
                        out.h2("Input")?;
                        let out = out.indent();
                        out.toolschema(&tool.input_schema)?;
                    }
                }

                // Output schema
                if let Some(output_schema) = &tool.output_schema {
                    if let Some(properties) = &output_schema.properties {
                        if !properties.is_empty() {
                            let out = out.indent();
                            out.h2("Output")?;
                            let out = out.indent();
                            out.toolschema(output_schema)?;
                        }
                    }
                }

                output.text("")?; // Extra blank line between tools
            }
        }
    }
    Ok(())
}
