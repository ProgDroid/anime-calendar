use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "queries/get_items.graphql",
    schema_path = "schemas/items.graphql",
    response_derives = "Debug,Clone"
)]
pub struct GetItems;
