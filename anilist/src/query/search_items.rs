use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "queries/search_items.graphql",
    schema_path = "schemas/search.graphql",
    response_derives = "Debug,Clone"
)]
pub struct SearchItems;
