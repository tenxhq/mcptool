use crate::Result;
use crate::output::Output;

/// Display the list of resource templates in either JSON or formatted text
pub fn list_resource_templates_result(
    output: &Output,
    templates_result: &tenx_mcp::schema::ListResourceTemplatesResult,
) -> Result<()> {
    if output.json {
        // Output as JSON
        output.json_value(templates_result)?;
    } else {
        // Output as formatted text
        if templates_result.resource_templates.is_empty() {
            output.text("No resource templates.")?;
        } else {
            for template in &templates_result.resource_templates {
                output.h1(&template.name)?;
                output.text("")?; // Extra blank line between templates

                // URI Template
                output.text(format!("URI Template: {}", template.uri_template))?;

                // Title (if present)
                if let Some(title) = &template.title {
                    output.text(format!("Title: {title}"))?;
                }

                // Description (if present)
                if let Some(description) = &template.description {
                    output.text(format!("Description: {description}"))?;
                }

                // MIME Type (if present)
                if let Some(mime_type) = &template.mime_type {
                    output.text(format!("MIME Type: {mime_type}"))?;
                }

                // Annotations (if present)
                if let Some(annotations) = &template.annotations {
                    output.text("")?; // Blank line before annotations
                    output.text("Annotations:")?;

                    if let Some(audience) = &annotations.audience {
                        output.text(format!("  Audience: {audience:?}"))?;
                    }
                    if let Some(priority) = &annotations.priority {
                        output.text(format!("  Priority: {priority}"))?;
                    }
                    if let Some(last_modified) = &annotations.last_modified {
                        output.text(format!("  Last Modified: {last_modified}"))?;
                    }
                }

                output.text("")?; // Extra blank line between templates
            }

            // Show pagination cursor if there are more templates
            if let Some(cursor) = &templates_result.next_cursor {
                output.text("")?;
                output.text(format!(
                    "More resource templates available. Next cursor: {cursor}"
                ))?;
            }
        }
    }
    Ok(())
}
