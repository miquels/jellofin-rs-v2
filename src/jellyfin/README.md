
# API Handlers.

All files in this directory correspond to a tag in the openapi spec.
For example, the handlers for the API paths documented under the tag 
Branding" are all in branding.rs.

There are three exceptions:

- util/: contains shared functionality.
- stubs.rs: contains handlers for endpoints that are implemented
  just as stubs, because clients expect them to exist.
- types.rs: common types and models.
