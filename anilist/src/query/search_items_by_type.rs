use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "queries/search_items_by_type.graphql",
    schema_path = "schemas/search_type.graphql",
    response_derives = "Debug"
)]
pub struct SearchItemsByType;
