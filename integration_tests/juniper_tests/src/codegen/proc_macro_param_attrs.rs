use juniper::*;
use serde_json::{self, Value};

struct Query;

#[juniper::object]
impl Query {
    #[graphql(arguments(
        arg1(default = true, description = "arg1 desc"),
        arg2(default = false, description = "arg2 desc"),
    ))]
    fn field_old_attrs(arg1: bool, arg2: bool) -> bool {
        arg1 && arg2
    }

    fn field_new_attrs(
        #[graphql(default = true, description = "arg1 desc")] arg1: bool,
        #[graphql(default = false, description = "arg2 desc")] arg2: bool,
    ) -> bool {
        arg1 && arg2
    }
}

// The query that GraphiQL runs to inspect the schema
static SCHEMA_INTROSPECTION_QUERY: &str = r#"
    query IntrospectionQuery {
      __schema {
        types {
          ...FullType
        }
      }
    }

    fragment FullType on __Type {
      name
      fields(includeDeprecated: true) {
        name
        description
        args {
          ...InputValue
        }
      }
    }

    fragment InputValue on __InputValue {
      name
      description
      defaultValue
    }
"#;

#[test]
fn old_descriptions_applied_correctly() {
    let schema = introspect_schema();
    let query = schema.types.iter().find(|ty| ty.name == "Query").unwrap();

    let field = query
        .fields
        .iter()
        .find(|field| field.name == "fieldOldAttrs")
        .unwrap();

    let arg1 = field.args.iter().find(|arg| arg.name == "arg1").unwrap();
    assert_eq!(&arg1.description, &Some("arg1 desc".to_string()));
    assert_eq!(
        &arg1.default_value,
        &Some(Value::String("true".to_string()))
    );

    let arg2 = field.args.iter().find(|arg| arg.name == "arg2").unwrap();
    assert_eq!(&arg2.description, &Some("arg2 desc".to_string()));
    assert_eq!(
        &arg2.default_value,
        &Some(Value::String("false".to_string()))
    );
}

#[test]
fn new_descriptions_applied_correctly() {
    let schema = introspect_schema();
    let query = schema.types.iter().find(|ty| ty.name == "Query").unwrap();

    let field = query
        .fields
        .iter()
        .find(|field| field.name == "fieldNewAttrs")
        .unwrap();

    let arg1 = field.args.iter().find(|arg| arg.name == "arg1").unwrap();
    assert_eq!(&arg1.description, &Some("arg1 desc".to_string()));
    assert_eq!(
        &arg1.default_value,
        &Some(Value::String("true".to_string()))
    );

    let arg2 = field.args.iter().find(|arg| arg.name == "arg2").unwrap();
    assert_eq!(&arg2.description, &Some("arg2 desc".to_string()));
    assert_eq!(
        &arg2.default_value,
        &Some(Value::String("false".to_string()))
    );
}

#[derive(Debug)]
struct Schema {
    types: Vec<Type>,
}

#[derive(Debug)]
struct Type {
    name: String,
    fields: Vec<Field>,
}

#[derive(Debug)]
struct Field {
    name: String,
    args: Vec<Arg>,
    description: Option<String>,
}

#[derive(Debug)]
struct Arg {
    name: String,
    description: Option<String>,
    default_value: Option<Value>,
}

fn introspect_schema() -> Schema {
    let (value, _errors) = juniper::execute(
        SCHEMA_INTROSPECTION_QUERY,
        None,
        &RootNode::new(Query, juniper::EmptyMutation::new()),
        &Variables::new(),
        &(),
    )
    .unwrap();

    let value: Value = serde_json::from_str(&serde_json::to_string(&value).unwrap()).unwrap();

    let types = value["__schema"]["types"]
        .as_array()
        .unwrap()
        .iter()
        .map(parse_type)
        .collect::<Vec<_>>();

    Schema { types }
}

fn parse_type(value: &Value) -> Type {
    let name = value["name"].as_str().unwrap().to_string();

    let fields = if value["fields"].is_null() {
        vec![]
    } else {
        value["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(parse_field)
            .collect::<Vec<_>>()
    };

    Type { name, fields }
}

fn parse_field(value: &Value) -> Field {
    let name = value["name"].as_str().unwrap().to_string();

    let description = if value["description"].is_null() {
        None
    } else {
        Some(value["description"].as_str().unwrap().to_string())
    };

    let args = value["args"]
        .as_array()
        .unwrap()
        .iter()
        .map(parse_arg)
        .collect::<Vec<_>>();

    Field {
        name,
        description,
        args,
    }
}

fn parse_arg(value: &Value) -> Arg {
    let name = value["name"].as_str().unwrap().to_string();

    let description = if value["description"].is_null() {
        None
    } else {
        Some(value["description"].as_str().unwrap().to_string())
    };

    let default_value = if value["defaultValue"].is_null() {
        None
    } else {
        Some(value["defaultValue"].clone())
    };

    Arg {
        name,
        description,
        default_value,
    }
}
