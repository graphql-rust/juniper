use rustc_serialize::json::{ToJson, Json};

use ::{GraphQLError, Value};
use ast::InputValue;
use executor::ExecutionError;
use parser::{ParseError, Spanning, SourcePosition};
use validation::RuleError;

fn parse_error_to_json(err: &Spanning<ParseError>) -> Json {
    Json::Array(vec![
        Json::Object(vec![
            ("message".to_owned(), format!("{}", err.item).to_json()),
            ("locations".to_owned(), vec![
                Json::Object(vec![
                    ("line".to_owned(), (err.start.line() + 1).to_json()),
                    ("column".to_owned(), (err.start.column() + 1).to_json())
                ].into_iter().collect()),
            ].to_json()),
        ].into_iter().collect()),
    ])
}

impl ToJson for ExecutionError {
    fn to_json(&self) -> Json {
        Json::Object(vec![
            ("message".to_owned(), self.message().to_json()),
            ("locations".to_owned(), vec![self.location().clone()].to_json()),
            ("path".to_owned(), self.path().to_json()),
        ].into_iter().collect())
    }
}

impl<'a> ToJson for GraphQLError<'a> {
    fn to_json(&self) -> Json {
        match *self {
            GraphQLError::ParseError(ref err) => parse_error_to_json(err),
            GraphQLError::ValidationError(ref errs) => errs.to_json(),
            GraphQLError::MultipleOperationsProvided => Json::String(
                "Must provide operation name if query contains multiple operations".to_owned()),
            GraphQLError::NoOperationProvided => Json::String(
                "Must provide an operation".to_owned()),
            GraphQLError::UnknownOperationName => Json::String(
                "Unknown operation".to_owned()),
        }
    }
}

impl ToJson for InputValue {
    fn to_json(&self) -> Json {
        match *self {
            InputValue::Null | InputValue::Variable(_) => Json::Null,
            InputValue::Int(i) => Json::I64(i),
            InputValue::Float(f) => Json::F64(f),
            InputValue::String(ref s) | InputValue::Enum(ref s) => Json::String(s.clone()),
            InputValue::Boolean(b) => Json::Boolean(b),
            InputValue::List(ref l) => Json::Array(l.iter().map(|x| x.item.to_json()).collect()),
            InputValue::Object(ref o) => Json::Object(o.iter().map(|&(ref k, ref v)| (k.item.clone(), v.item.to_json())).collect()),
       }
    }
}

impl InputValue {
    /// Convert a `Json` structure into an `InputValue`.
    ///
    /// This consumes the JSON instance.
    ///
    /// Notes:
    /// * No enums or variables will be produced by this method.
    /// * All lists and objects will be unlocated
    pub fn from_json(json: Json) -> InputValue {
        match json {
            Json::I64(i) => InputValue::int(i),
            Json::U64(u) => InputValue::float(u as f64),
            Json::F64(f) => InputValue::float(f),
            Json::String(s) => InputValue::string(s),
            Json::Boolean(b) => InputValue::boolean(b),
            Json::Array(a) => InputValue::list(a.into_iter().map(InputValue::from_json).collect()),
            Json::Object(o) => InputValue::object(o.into_iter().map(|(k, v)| (k, InputValue::from_json(v))).collect()),
            Json::Null => InputValue::null(),
        }
    }
}

impl ToJson for RuleError {
    fn to_json(&self) -> Json {
        Json::Object(vec![
            ("message".to_owned(), self.message().to_json()),
            ("locations".to_owned(), self.locations().to_json()),
        ].into_iter().collect())
    }
}

impl ToJson for SourcePosition {
    fn to_json(&self) -> Json {
        Json::Object(vec![
            ("line".to_owned(), (self.line() + 1).to_json()),
            ("column".to_owned(), (self.column() + 1).to_json()),
        ].into_iter().collect())
    }
}

impl ToJson for Value {
    fn to_json(&self) -> Json {
        match *self {
            Value::Null => Json::Null,
            Value::Int(i) => Json::I64(i),
            Value::Float(f) => Json::F64(f),
            Value::String(ref s) => Json::String(s.clone()),
            Value::Boolean(b) => Json::Boolean(b),
            Value::List(ref l) => Json::Array(l.iter().map(|x| x.to_json()).collect()),
            Value::Object(ref o) => Json::Object(o.iter().map(|(k,v)| (k.clone(), v.to_json())).collect()),
       }
    }
}
