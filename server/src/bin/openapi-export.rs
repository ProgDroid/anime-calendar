//! Print the utoipa-generated `OpenAPI` spec as pretty JSON to stdout.
//!
//! Used by CI to materialize the spec for redocly-cli linting without
//! standing up the server or a database. Locally:
//!
//! ```sh
//! cargo run --quiet --bin openapi-export > openapi.json
//! ```

use server::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() {
    let json = ApiDoc::openapi()
        .to_pretty_json()
        .expect("OpenAPI spec should serialize to JSON");
    println!("{json}");
}
