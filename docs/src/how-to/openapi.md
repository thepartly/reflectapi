# OpenAPI Integration

Learn how to generate OpenAPI specifications from your ReflectAPI schema and serve interactive API documentation.

## Overview

ReflectAPI automatically generates OpenAPI 3.0 specifications from your API schema. This allows you to:
- Generate interactive API documentation
- Import your API into tools like Postman or Insomnia
- Use OpenAPI generators for additional client libraries
- Integrate with API gateways and management platforms

## Generating OpenAPI Specification

### Basic Generation

Convert your ReflectAPI schema to OpenAPI format:

```rust,ignore
use reflectapi::{Builder, codegen::openapi::Spec};

// Build your API
let builder = Builder::new()
    .name("My API")
    .description("API for managing resources")
    .version("1.0.0");
    // ... add routes

let (schema, _routers) = builder.build()?;

// Generate OpenAPI spec
let openapi_spec = Spec::from(&schema);

// Save to file
let openapi_json = serde_json::to_string_pretty(&openapi_spec)?;
std::fs::write("openapi.json", openapi_json)?;
```

### Customizing OpenAPI Output

Configure the OpenAPI generation:

```rust,ignore
use reflectapi::codegen::openapi::{Spec, Info, Server};

// Create custom OpenAPI info
let mut spec = Spec::from(&schema);

// Customize API information
spec.info = Info {
    title: "My API".to_string(),
    description: Some("Production API for resource management".to_string()),
    version: "2.0.0".to_string(),
    terms_of_service: Some("https://example.com/terms".to_string()),
    contact: Some(Contact {
        name: Some("API Support".to_string()),
        email: Some("api@example.com".to_string()),
        url: Some("https://support.example.com".to_string()),
    }),
    license: Some(License {
        name: "Apache 2.0".to_string(),
        url: Some("https://www.apache.org/licenses/LICENSE-2.0".to_string()),
    }),
};

// Add server endpoints
spec.servers = vec![
    Server {
        url: "https://api.example.com".to_string(),
        description: Some("Production server".to_string()),
    },
    Server {
        url: "https://staging-api.example.com".to_string(),
        description: Some("Staging server".to_string()),
    },
];
```

## Serving Interactive Documentation

### Swagger UI Integration

Serve Swagger UI for interactive API exploration:

```rust,ignore
use axum::{response::Html, Json, routing::get};

// Create Swagger UI HTML template
const SWAGGER_UI_HTML: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>API Documentation</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui.css">
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script>
        window.onload = () => {
            window.ui = SwaggerUIBundle({
                url: '/openapi.json',
                dom_id: '#swagger-ui',
                deepLinking: true,
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIBundle.SwaggerUIStandalonePreset
                ],
                layout: "BaseLayout"
            });
        };
    </script>
</body>
</html>
"#;

// Add routes to your Axum app
let app = axum_app
    .route("/openapi.json", get(move || async move {
        Json(openapi_spec.clone())
    }))
    .route("/doc", get(|| async {
        Html(SWAGGER_UI_HTML)
    }));
```

### ReDoc Integration

Use ReDoc for a cleaner, more modern documentation interface:

```rust,ignore
const REDOC_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>API Documentation</title>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        body { margin: 0; padding: 0; }
    </style>
</head>
<body>
    <redoc spec-url='/openapi.json'></redoc>
    <script src="https://cdn.redoc.ly/redoc/latest/bundles/redoc.standalone.js"></script>
</body>
</html>
"#;

// Add ReDoc route
let app = axum_app
    .route("/redoc", get(|| async {
        Html(REDOC_HTML)
    }));
```

## Working with OpenAPI Schema

### Schema Enrichment

ReflectAPI preserves documentation from your Rust code:

```rust,ignore
/// User account information
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct User {
    /// Unique identifier
    pub id: u32,
    
    /// User's email address
    /// 
    /// Must be unique across the system
    #[serde(rename = "email_address")]
    pub email: String,
    
    /// User's display name
    pub name: String,
    
    /// Account creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}
```

This documentation appears in the OpenAPI spec:

```json
{
  "components": {
    "schemas": {
      "User": {
        "type": "object",
        "description": "User account information",
        "properties": {
          "id": {
            "type": "integer",
            "format": "int32",
            "description": "Unique identifier"
          },
          "email_address": {
            "type": "string",
            "description": "User's email address\n\nMust be unique across the system"
          },
          "name": {
            "type": "string",
            "description": "User's display name"
          },
          "created_at": {
            "type": "string",
            "format": "date-time",
            "description": "Account creation timestamp"
          }
        },
        "required": ["id", "email_address", "name", "created_at"]
      }
    }
  }
}
```

### Adding Examples

Include examples in your OpenAPI specification:

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    #[serde(example = "user@example.com")]
    pub email: String,
    
    #[serde(example = "John Doe")]
    pub name: String,
    
    #[serde(example = "1990-01-01")]
    pub birth_date: String,
}
```

### Security Schemes

Add authentication to your OpenAPI spec:

```rust,ignore
use reflectapi::codegen::openapi::{SecurityScheme, ApiKeyLocation};

// Add security schemes
spec.components.security_schemes.insert(
    "ApiKey".to_string(),
    SecurityScheme::ApiKey {
        name: "X-API-Key".to_string(),
        location: ApiKeyLocation::Header,
    },
);

spec.components.security_schemes.insert(
    "Bearer".to_string(),
    SecurityScheme::Http {
        scheme: "bearer".to_string(),
        bearer_format: Some("JWT".to_string()),
    },
);

// Apply security to all operations
spec.security = vec![
    HashMap::from([("ApiKey".to_string(), vec![])]),
    HashMap::from([("Bearer".to_string(), vec![])]),
];
```

## Validation and Testing

### Validate OpenAPI Spec

Use external tools to validate your generated spec:

```bash
# Install OpenAPI validator
npm install -g @apidevtools/swagger-cli

# Validate the generated spec
swagger-cli validate openapi.json
```

### Import to API Testing Tools

Your generated OpenAPI spec can be imported into:
- **Postman**: Import → File → Upload openapi.json
- **Insomnia**: Import → From File → openapi.json
- **Bruno**: Import Collection → OpenAPI Spec
- **Thunder Client**: Import → OpenAPI

## Complete Example

Here's a complete example combining all features:

```rust,ignore
use axum::{response::Html, Json};
use reflectapi::codegen::openapi::Spec;

pub async fn setup_openapi_docs(
    schema: reflectapi::Schema,
    app: axum::Router,
) -> axum::Router {
    // Generate OpenAPI spec
    let mut openapi_spec = Spec::from(&schema);
    
    // Customize metadata
    openapi_spec.info.title = "My Production API".to_string();
    openapi_spec.info.version = env!("CARGO_PKG_VERSION").to_string();
    openapi_spec.info.description = Some(
        "Production API with full OpenAPI documentation".to_string()
    );
    
    // Add servers
    openapi_spec.servers = vec![
        Server {
            url: "https://api.example.com".to_string(),
            description: Some("Production".to_string()),
        },
        Server {
            url: "http://localhost:3000".to_string(),
            description: Some("Local development".to_string()),
        },
    ];
    
    // Save to file for external tools
    tokio::fs::write(
        "openapi.json",
        serde_json::to_string_pretty(&openapi_spec)?,
    ).await?;
    
    // Add documentation routes
    app
        // OpenAPI JSON endpoint
        .route("/openapi.json", axum::routing::get({
            let spec = openapi_spec.clone();
            move || async move { Json(spec) }
        }))
        // Swagger UI
        .route("/swagger", axum::routing::get(|| async {
            Html(include_str!("../static/swagger-ui.html"))
        }))
        // ReDoc
        .route("/docs", axum::routing::get(|| async {
            Html(include_str!("../static/redoc.html"))
        }))
        // Redirect root to docs
        .route("/", axum::routing::get(|| async {
            axum::response::Redirect::permanent("/docs")
        }))
}
```

## Best Practices

1. **Version Your API**: Use semantic versioning in your OpenAPI spec
2. **Document Everything**: Add descriptions to all types, fields, and endpoints
3. **Include Examples**: Provide realistic examples for request/response bodies
4. **Security First**: Document all authentication and authorization requirements
5. **Test Integration**: Regularly import your spec into API testing tools
6. **Automate Updates**: Generate OpenAPI spec as part of your build process

## Troubleshooting

### Common Issues

**Missing Documentation**: Ensure you're using doc comments (`///`) on your Rust types:
```rust,ignore
/// This comment appears in OpenAPI
struct MyType { ... }

// This comment does NOT appear
struct MyType { ... }
```

**Schema Not Found**: Make sure to register all types with the builder:
```rust,ignore
builder
    .input::<CreateUserRequest>()
    .output::<User>()
```

**Invalid OpenAPI**: Validate with external tools and check for:
- Missing required fields
- Invalid schema references
- Circular dependencies

## Next Steps

- Learn about [Adding Middleware](./middleware.md) for API authentication
- Explore [Client Generation](./clients.md) from OpenAPI specs
- See [Testing Your API](../tutorial/testing.md) with generated documentation