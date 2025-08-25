use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "queries/get_item.graphql",
    schema_path = "schemas/item.graphql",
    response_derives = "Debug"
)]
pub struct GetItem;
