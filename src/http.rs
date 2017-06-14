use serde::ser;
use serde::ser::SerializeMap;

use ::{GraphQLError, Value, Variables, GraphQLType, RootNode};
use ast::InputValue;
use executor::ExecutionError;

/// The expected structure of the decoded JSON Document for either Post or Get requests.
#[derive(Deserialize)]
pub struct GraphQLRequest {
    query: String,
    #[serde(rename = "operationName")]
    operation_name: Option<String>,
    variables: Option<InputValue>
}

impl GraphQLRequest {
    fn operation_name(&self) -> Option<&str> {
        self.operation_name.as_ref().map(|oper_name| &**oper_name)
    }

    fn variables(&self) -> Variables {
        self.variables.as_ref().and_then(|iv| {
            iv.to_object_value().map(|o| {
                o.into_iter().map(|(k, v)| (k.to_owned(), v.clone())).collect()
            })
        }).unwrap_or_default()
    }

    pub fn new(query: String, operation_name: Option<String>, variables: Option<InputValue>) -> GraphQLRequest {
        GraphQLRequest {
            query: query,
            operation_name: operation_name,
            variables: variables,
        }
    }

    pub fn execute<'a, CtxT, QueryT, MutationT>(
        &'a self,
        root_node: &RootNode<QueryT, MutationT>,
        context: &CtxT,
    )
        -> GraphQLResponse<'a>
        where QueryT: GraphQLType<Context=CtxT>,
            MutationT: GraphQLType<Context=CtxT>,
    {
        GraphQLResponse(::execute(
            &self.query,
            self.operation_name(),
            root_node,
            &self.variables(),
            context,
        ))
    }
}


pub struct GraphQLResponse<'a>(Result<(Value, Vec<ExecutionError>), GraphQLError<'a>>);

impl<'a> GraphQLResponse<'a> {
    pub fn is_ok(&self) -> bool {
        self.0.is_ok()
    }
}

impl<'a> ser::Serialize for GraphQLResponse<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: ser::Serializer,
    {
        match self.0 {
            Ok((ref res, ref err)) => {
                let mut map = try!(serializer.serialize_map(None));

                try!(map.serialize_key("data"));
                try!(map.serialize_value(res));

                if !err.is_empty() {
                    try!(map.serialize_key("errors"));
                    try!(map.serialize_value(err));
                }

                map.end()
            },
            Err(ref err) => {
                let mut map = try!(serializer.serialize_map(Some(1)));
                try!(map.serialize_key("errors"));
                try!(map.serialize_value(err));
                map.end()
            },
        }
    }
}
