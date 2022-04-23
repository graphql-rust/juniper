//! GraphQL support for [`serde_json::Value`].

use std::{
    convert::{TryFrom as _, TryInto as _},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::AtomicPtr,
};

use futures::future;
use graphql_parser::{
    query::Type as SchemaType,
    schema::{Document as Schema, ParseError},
};
use ref_cast::RefCast;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    ast,
    macros::reflect,
    marker::{IsInputType, IsOutputType},
    meta::{self, MetaType},
    parser::ScalarToken,
    Arguments, BoxFuture, ExecutionResult, Executor, FieldError, FromInputValue, GraphQLType,
    GraphQLValue, GraphQLValueAsync, InputValue, IntoFieldError, ParseScalarResult,
    ParseScalarValue, Registry, ScalarValue, Selection, Spanning, ToInputValue, ID,
};

pub use serde_json::{json, Error, Value};

impl<S: ScalarValue> IntoFieldError<S> for Error {
    fn into_field_error(self) -> FieldError<S> {
        self.into()
    }
}

impl<S: ScalarValue> IsInputType<S> for Value {}

impl<S: ScalarValue> IsOutputType<S> for Value {}

impl<S: ScalarValue> GraphQLType<S> for Value {
    fn name(info: &Self::TypeInfo) -> Option<&str> {
        <Json as GraphQLType<S>>::name(info)
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        <Json as GraphQLType<S>>::meta(info, registry)
    }
}

impl<S: ScalarValue> GraphQLValue<S> for Value {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType<S>>::name(info)
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        selection: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        use serde::ser::Error as _;

        if selection.is_some() && matches!(self, Self::Bool(_) | Self::Number(_) | Self::String(_))
        {
            return Err(FieldError::new(
                "cannot select fields on a leaf opaque JSON value",
                crate::Value::null(),
            ));
        }

        match self {
            Self::Null => Ok(crate::Value::null()),
            Self::Bool(b) => executor.resolve(&(), &b),
            Self::Number(n) => {
                if let Some(n) = n.as_u64() {
                    executor.resolve::<i32>(&(), &n.try_into().map_err(Error::custom)?)
                } else if let Some(n) = n.as_i64() {
                    executor.resolve::<i32>(&(), &n.try_into().map_err(Error::custom)?)
                } else if let Some(n) = n.as_f64() {
                    executor.resolve(&(), &n)
                } else {
                    return Err(
                        Error::custom("`serde_json::Number` has only 3 number variants").into(),
                    );
                }
            }
            Self::String(s) => executor.resolve(&(), &s),
            Self::Array(arr) => Ok(crate::Value::list(
                arr.iter()
                    .map(|v| executor.resolve(info, v))
                    .collect::<Result<_, _>>()?,
            )),
            Self::Object(obj) => {
                // If selection set is none we should still output all the
                // object fields.
                let full_selection = selection
                    .is_none()
                    .then(|| {
                        obj.keys()
                            .map(|k| {
                                Selection::Field(Spanning::unlocated(ast::Field {
                                    alias: None,
                                    name: Spanning::unlocated(&*k),
                                    arguments: None,
                                    directives: None,
                                    selection_set: None,
                                }))
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let selection = selection.unwrap_or(&full_selection);

                let mut out = crate::Object::with_capacity(selection.len());
                for sel in selection {
                    match sel {
                        Selection::Field(Spanning {
                            item: f,
                            start: start_pos,
                            ..
                        }) => {
                            let resp_name = f.alias.as_ref().unwrap_or(&f.name).item;
                            let sub_exec = executor.field_with_parent_type_sub_executor(
                                resp_name,
                                *start_pos,
                                f.selection_set.as_ref().map(|v| &v[..]),
                            );
                            let _ = out.add_field(
                                resp_name,
                                self.resolve_field(
                                    info,
                                    f.name.item,
                                    &Arguments::new(None, &None),
                                    &sub_exec,
                                )?,
                            );
                        }
                        _ => {
                            return Err(FieldError::new(
                                "spreading fragments on opaque JSON value is \
                                 not supported",
                                crate::Value::null(),
                            ))
                        }
                    }
                }
                Ok(crate::Value::Object(out))
            }
        }
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field_name: &str,
        _: &Arguments<S>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        match self {
            Self::Object(obj) => match obj.get(field_name) {
                None => Ok(crate::Value::null()),
                Some(field) => executor.resolve(info, field),
            },
            _ => Err(FieldError::new("not an object value", crate::Value::null())),
        }
    }
}

impl<S: ScalarValue + Send + Sync> GraphQLValueAsync<S> for Value {
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        selection: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        Box::pin(future::ready(self.resolve(info, selection, executor)))
    }

    fn resolve_field_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        field_name: &'a str,
        arguments: &'a Arguments<S>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        Box::pin(future::ready(
            self.resolve_field(info, field_name, arguments, executor),
        ))
    }
}

impl<S: ScalarValue> ToInputValue<S> for Value {
    fn to_input_value(&self) -> InputValue<S> {
        match self {
            Self::Null => InputValue::null(),
            Self::Bool(b) => InputValue::scalar(*b),
            Self::Number(n) => {
                if let Some(n) = n.as_u64() {
                    InputValue::scalar(i32::try_from(n).expect("`i32` number"))
                } else if let Some(n) = n.as_i64() {
                    InputValue::scalar(i32::try_from(n).expect("`i32` number"))
                } else if let Some(n) = n.as_f64() {
                    InputValue::scalar(n)
                } else {
                    unreachable!("`serde_json::Number` has only 3 number variants")
                }
            }
            Self::String(s) => InputValue::scalar(s.clone()),
            Self::Array(arr) => InputValue::list(arr.iter().map(Self::to_input_value).collect()),
            Self::Object(obj) => {
                InputValue::object(obj.iter().map(|(k, v)| (k, v.to_input_value())).collect())
            }
        }
    }
}

impl<S: ScalarValue> FromInputValue<S> for Value {
    type Error = FieldError<S>;

    fn from_input_value(val: &InputValue<S>) -> Result<Self, Self::Error> {
        match val {
            InputValue::Null => Ok(Self::Null),
            InputValue::Scalar(x) => Ok(if let Some(i) = x.as_int() {
                Self::Number(serde_json::Number::from(i))
            } else if let Some(f) = x.as_float() {
                Self::Number(serde_json::Number::from_f64(f).ok_or_else(|| {
                    format!(
                        "`serde_json::Number` cannot be created from invalid \
                         `f64` value: {f}",
                    )
                })?)
            } else if let Some(b) = x.as_bool() {
                Self::Bool(b)
            } else if let Some(s) = x.as_str() {
                Self::String(s.into())
            } else {
                return Err("`ScalarValue` must represent at least one of the \
                            GraphQL spec types"
                    .into());
            }),
            InputValue::Enum(x) => Ok(Self::String(x.clone())),
            InputValue::List(ls) => Ok(Self::Array(
                ls.iter()
                    .map(|i| i.item.convert())
                    .collect::<Result<_, _>>()?,
            )),
            InputValue::Object(fs) => Ok(Self::Object(
                fs.iter()
                    .map(|(n, v)| v.item.convert().map(|v| (n.item.clone(), v)))
                    .collect::<Result<_, _>>()?,
            )),
            InputValue::Variable(_) => {
                Err("`serde_json::Value` cannot be created from GraphQL variable".into())
            }
        }
    }
}

impl<S: ScalarValue> ParseScalarValue<S> for Value {
    fn from_str(val: ScalarToken<'_>) -> ParseScalarResult<'_, S> {
        match val {
            ScalarToken::String(_) => <String as ParseScalarValue<S>>::from_str(val),
            ScalarToken::Float(_) => <f64 as ParseScalarValue<S>>::from_str(val),
            ScalarToken::Int(_) => <i32 as ParseScalarValue<S>>::from_str(val),
        }
    }
}

impl ScalarValue for Value {
    fn as_int(&self) -> Option<i32> {
        match self {
            Self::Number(n) => (n.as_u64().map(i32::try_from))
                .or_else(|| n.as_i64().map(i32::try_from))
                .transpose()
                .expect("`i32` number"),
            _ => None,
        }
    }

    fn as_float(&self) -> Option<f64> {
        match self {
            Self::Number(n) => (n.as_u64().map(|u| u as f64))
                .or_else(|| n.as_i64().map(|i| i as f64))
                .or_else(|| n.as_f64()),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    fn as_string(&self) -> Option<String> {
        match self {
            Self::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    fn into_string(self) -> Option<String> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    fn into_another<S: ScalarValue>(self) -> S {
        match self {
            Self::Bool(b) => S::from(b),
            Self::Number(n) => {
                if let Some(n) = n.as_u64() {
                    S::from(i32::try_from(n).expect("`i32` number"))
                } else if let Some(n) = n.as_i64() {
                    S::from(i32::try_from(n).expect("`i32` number"))
                } else if let Some(n) = n.as_f64() {
                    S::from(n)
                } else {
                    unreachable!("`serde_json::Number` has only 3 number variants")
                }
            }
            Self::String(s) => S::from(s),
            _ => unreachable!("not a leaf `serde_json::Value`"),
        }
    }
}

impl<S: ScalarValue> From<crate::Value<S>> for Value {
    fn from(val: crate::Value<S>) -> Self {
        match val {
            crate::Value::Null => Self::Null,
            crate::Value::Scalar(s) => s.into_another(),
            crate::Value::List(l) => Self::Array(l.into_iter().map(Self::from).collect()),
            crate::Value::Object(o) => {
                Self::Object(o.into_iter().map(|(k, v)| (k, Self::from(v))).collect())
            }
        }
    }
}

impl<S: ScalarValue> From<Value> for crate::Value<S> {
    fn from(val: Value) -> Self {
        match val {
            Value::Null => Self::Null,
            Value::Array(a) => Self::List(a.into_iter().map(Self::from).collect()),
            Value::Object(o) => {
                Self::Object(o.into_iter().map(|(k, v)| (k, Self::from(v))).collect())
            }
            s @ (Value::Bool(_) | Value::Number(_) | Value::String(_)) => {
                Self::Scalar(s.into_another())
            }
        }
    }
}

impl<S> reflect::BaseType<S> for Value {
    const NAME: reflect::Type = "Json";
}

impl<S> reflect::BaseSubTypes<S> for Value {
    const NAMES: reflect::Types = &[<Self as reflect::BaseType<S>>::NAME];
}

impl<S> reflect::WrappedType<S> for Value {
    const VALUE: reflect::WrappedValue = 1;
}

#[derive(Clone, Deserialize, Copy, Debug, RefCast, Serialize)]
#[repr(transparent)]
pub struct Json<T: ?Sized = Value, I: ?Sized = ()> {
    _type: PhantomData<AtomicPtr<Box<I>>>,
    val: T,
}

impl<T, I: ?Sized> From<T> for Json<T, I> {
    fn from(val: T) -> Self {
        Self {
            _type: PhantomData,
            val,
        }
    }
}

impl<T, I: ?Sized> Json<T, I> {
    /// Wraps the given `value` into a [`Json`] wrapper.
    #[must_use]
    pub fn wrap(value: T) -> Self {
        value.into()
    }

    /// Unwraps into the underlying value of this [`Json`] wrapper.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.val
    }

    /// Maps the inner value of this [`Json`] wrapper with the given function.
    #[must_use]
    pub fn map<To>(self, f: impl FnOnce(T) -> To) -> Json<To, I> {
        Json {
            _type: PhantomData,
            val: f(self.val),
        }
    }
}

impl<T: ?Sized, I: ?Sized> Deref for Json<T, I> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

impl<T: ?Sized, I: ?Sized> DerefMut for Json<T, I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.val
    }
}

impl<T, I, S> IsInputType<S> for Json<T, I>
where
    T: DeserializeOwned + Serialize + ?Sized,
    I: TypeInfo + ?Sized,
    S: ScalarValue,
{
}

impl<T, I, S> IsOutputType<S> for Json<T, I>
where
    T: DeserializeOwned + Serialize + ?Sized,
    I: TypeInfo + ?Sized,
    S: ScalarValue,
{
}

impl<T, I, S> GraphQLType<S> for Json<T, I>
where
    T: DeserializeOwned + Serialize + ?Sized,
    I: TypeInfo + ?Sized,
    S: ScalarValue,
{
    fn name(info: &Self::TypeInfo) -> Option<&str> {
        Some(info.name())
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        info.meta::<Self, S>(registry)
    }
}

impl<T, I, S> GraphQLValue<S> for Json<T, I>
where
    T: DeserializeOwned + Serialize + ?Sized,
    I: TypeInfo + ?Sized,
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = I;

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType<S>>::name(info)
    }

    fn resolve(
        &self,
        _: &Self::TypeInfo,
        selection: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        serde_json::to_value(&self.val)?.resolve(&(), selection, executor)
    }

    fn resolve_field(
        &self,
        _: &Self::TypeInfo,
        field_name: &str,
        args: &Arguments<S>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        serde_json::to_value(&self.val)?.resolve_field(&(), field_name, args, executor)
    }
}

impl<T, I, S> GraphQLValueAsync<S> for Json<T, I>
where
    T: DeserializeOwned + Serialize + Sync + ?Sized,
    I: TypeInfo + Sync + ?Sized,
    S: ScalarValue + Send + Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        selection: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        Box::pin(future::ready(self.resolve(info, selection, executor)))
    }

    fn resolve_field_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        field_name: &'a str,
        arguments: &'a Arguments<S>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        Box::pin(future::ready(
            self.resolve_field(info, field_name, arguments, executor),
        ))
    }
}

impl<T, I, S> ToInputValue<S> for Json<T, I>
where
    T: Serialize, // TODO: + ?Sized
    I: TypeInfo + ?Sized,
    S: ScalarValue,
{
    fn to_input_value(&self) -> InputValue<S> {
        serde_json::to_value(&self.val)
            .expect("Failed to serialize")
            .to_input_value()
    }
}

impl<T, I, S> FromInputValue<S> for Json<T, I>
where
    T: DeserializeOwned,
    I: TypeInfo + ?Sized,
    S: ScalarValue,
{
    type Error = FieldError<S>;

    fn from_input_value(val: &InputValue<S>) -> Result<Self, Self::Error> {
        Ok(Self::wrap(serde_json::from_value(
            <Value as FromInputValue<S>>::from_input_value(val)?,
        )?))
    }
}

impl<T, I, S> ParseScalarValue<S> for Json<T, I>
where
    T: ?Sized,
    I: TypeInfo + ?Sized,
    S: ScalarValue,
{
    fn from_str(val: ScalarToken<'_>) -> ParseScalarResult<'_, S> {
        <Value as ParseScalarValue<S>>::from_str(val)
    }
}

impl<T: ?Sized, I: ?Sized, S> reflect::BaseType<S> for Json<T, I> {
    const NAME: reflect::Type = "Json"; // TODO: json?
}

impl<T: ?Sized, I: ?Sized, S> reflect::BaseSubTypes<S> for Json<T, I> {
    const NAMES: reflect::Types = &[<Self as reflect::BaseType<S>>::NAME];
}

impl<T: ?Sized, I: ?Sized, S> reflect::WrappedType<S> for Json<T, I> {
    const VALUE: reflect::WrappedValue = 1;
}

pub trait TypeInfo {
    fn name(&self) -> &str;

    fn meta<'r, T, S>(&self, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        T: FromInputValue<S> + GraphQLType<S, TypeInfo = Self> + ?Sized,
        T::Error: IntoFieldError<S>,
        S: ScalarValue + 'r;
}

impl TypeInfo for () {
    fn name(&self) -> &str {
        <Value as reflect::BaseType<()>>::NAME
    }

    fn meta<'r, T, S>(&self, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        T: FromInputValue<S> + GraphQLType<S, TypeInfo = Self> + ?Sized,
        T::Error: IntoFieldError<S>,
        S: ScalarValue + 'r,
    {
        registry
            .build_scalar_type::<Value>(self)
            .description("Opaque JSON value.")
            .into_meta()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Info {
    /// Parsed [`Schema`] containing a definition of the GraphQL type.
    schema: Schema<'static, String>,

    /// Type name of a [`GraphQLValue`] using this [`Info`].
    name: String,
}

impl Info {
    pub fn try_new<N: Into<String>>(
        name: N,
        schema: Schema<'_, String>,
    ) -> Result<Self, ParseError> {
        let schema = schema.into_static();
        let name = name.into();

        // TODO: validate `name` is contained in `schema`.

        Ok(Self { schema, name })
    }

    pub fn parse<N: Into<String>, S: AsRef<str>>(name: N, sdl: S) -> Result<Self, ParseError> {
        Self::try_new(name, graphql_parser::parse_schema::<String>(sdl.as_ref())?)
    }

    /// Returns type name of a [`GraphQLValue`] using this [`Info`].
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns parsed [`Schema`] defining this [`Info`].
    #[must_use]
    pub fn schema(&self) -> &Schema<'static, String> {
        &self.schema
    }

    fn build_field<'r, 't, S>(
        &self,
        registry: &mut Registry<'r, S>,
        field_name: &str,
        ty: &SchemaType<'t, String>,
        nullable: bool,
    ) -> meta::Field<'r, S>
    where
        S: 'r + ScalarValue,
    {
        match ty {
            SchemaType::NamedType(n) => match n.as_ref() {
                "Boolean" => {
                    if nullable {
                        registry.field::<Option<bool>>(field_name, &())
                    } else {
                        registry.field::<bool>(field_name, &())
                    }
                }
                "Int" => {
                    if nullable {
                        registry.field::<Option<i32>>(field_name, &())
                    } else {
                        registry.field::<i32>(field_name, &())
                    }
                }
                "Float" => {
                    if nullable {
                        registry.field::<Option<f64>>(field_name, &())
                    } else {
                        registry.field::<f64>(field_name, &())
                    }
                }
                "String" => {
                    if nullable {
                        registry.field::<Option<String>>(field_name, &())
                    } else {
                        registry.field::<String>(field_name, &())
                    }
                }
                "ID" => {
                    if nullable {
                        registry.field::<Option<ID>>(field_name, &())
                    } else {
                        registry.field::<ID>(field_name, &())
                    }
                }
                _ => {
                    let field_type_info = Info {
                        schema: self.schema.clone(),
                        name: n.clone(),
                    };
                    if nullable {
                        registry.field::<Option<Json<Value, Info>>>(field_name, &field_type_info)
                    } else {
                        registry.field::<Json<Value, Info>>(field_name, &field_type_info)
                    }
                }
            },
            SchemaType::ListType(ty) => {
                let mut item = self.build_field(registry, field_name, &**ty, true);
                if nullable {
                    item.field_type = crate::Type::List(Box::new(item.field_type), None);
                } else {
                    item.field_type = crate::Type::NonNullList(Box::new(item.field_type), None);
                }
                item
            }
            SchemaType::NonNullType(ty) => self.build_field(registry, field_name, &**ty, false),
        }
    }
}

impl TypeInfo for Info {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn meta<'r, T, S>(&self, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        T: FromInputValue<S> + GraphQLType<S, TypeInfo = Self> + ?Sized,
        T::Error: IntoFieldError<S>,
        S: ScalarValue + 'r,
    {
        use graphql_parser::schema::{Definition, TypeDefinition};

        let mut fields = Vec::new();
        let mut input_fields = Vec::new();
        let mut is_input_object = false;

        for d in &self.schema.definitions {
            match &d {
                Definition::TypeDefinition(d) => match d {
                    TypeDefinition::Object(o) => {
                        if o.name == self.name {
                            for f in &o.fields {
                                fields.push(self.build_field(
                                    registry,
                                    &f.name,
                                    &f.field_type,
                                    true,
                                ));
                            }
                            break;
                        }
                    }
                    TypeDefinition::InputObject(o) => {
                        if o.name == self.name {
                            is_input_object = true;
                            for f in &o.fields {
                                let f = self.build_field(registry, &f.name, &f.value_type, true);
                                input_fields.push(meta::Argument {
                                    name: f.name.to_string(),
                                    description: f.description.clone(),
                                    arg_type: f.field_type,
                                    default_value: None,
                                });
                            }
                            break;
                        }
                    }
                    // We do just nothing in other cases, as at this point the
                    // `self.schema` has been validated already in
                    // `Info::parse()` to contain the necessary types.
                    _ => {}
                },
                _ => {}
            }
        }

        if is_input_object {
            registry
                .build_input_object_type::<T>(self, &input_fields)
                .into_meta()
        } else {
            registry.build_object_type::<T>(self, &fields).into_meta()
        }
    }
}

/// Dynamic [`Json`] value typed by an [`Info`].
pub type Typed<T = Value> = Json<T, Info>;

#[cfg(test)]
mod value_test {
    mod as_output {
        use futures::FutureExt as _;
        use serde_json::{json, Value};

        use crate::{
            execute, execute_sync, graphql_object, graphql_subscription, graphql_vars,
            resolve_into_stream,
            tests::util::{extract_next, stream, Stream},
            EmptyMutation, FieldResult, RootNode,
        };

        struct Query;

        #[graphql_object]
        impl Query {
            fn null() -> Value {
                Value::Null
            }

            fn bool() -> Value {
                json!(true)
            }

            fn int() -> Value {
                json!(42)
            }

            fn float() -> Value {
                json!(3.14)
            }

            fn string() -> Value {
                json!("Galadriel")
            }

            fn array() -> Value {
                json!(["Ai", "Ambarendya!"])
            }

            fn object() -> Value {
                json!({"message": ["Ai", "Ambarendya!"]})
            }

            fn nullable() -> Option<Value> {
                Some(json!({"message": ["Ai", "Ambarendya!"]}))
            }

            fn fallible() -> FieldResult<Value> {
                Ok(json!({"message": ["Ai", "Ambarendya!"]}))
            }

            fn nested() -> Value {
                json!({"message": {
                    "header": "Ai",
                    "body": "Ambarendya!",
                }})
            }
        }

        struct Subscription;

        #[graphql_subscription]
        impl Subscription {
            async fn null() -> Stream<Value> {
                stream(Value::Null)
            }

            async fn bool() -> Stream<Value> {
                stream(json!(true))
            }

            async fn int() -> Stream<Value> {
                stream(json!(42))
            }

            async fn float() -> Stream<Value> {
                stream(json!(3.14))
            }

            async fn string() -> Stream<Value> {
                stream(json!("Galadriel"))
            }

            async fn array() -> Stream<Value> {
                stream(json!(["Ai", "Ambarendya!"]))
            }

            async fn object() -> Stream<Value> {
                stream(json!({"message": ["Ai", "Ambarendya!"]}))
            }

            async fn nullable() -> Stream<Option<Value>> {
                stream(Some(json!({"message": ["Ai", "Ambarendya!"]})))
            }

            async fn fallible() -> FieldResult<Stream<FieldResult<Value>>> {
                Ok(stream(Ok(json!({"message": ["Ai", "Ambarendya!"]}))))
            }

            async fn nested() -> Stream<Value> {
                stream(json!({"message": {
                    "header": "Ai",
                    "body": "Ambarendya!",
                }}))
            }
        }

        #[tokio::test]
        async fn resolves_null() {
            const QRY: &str = "{ null }";
            const SUB: &str = "subscription { null }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((graphql_value!({ "null": null }), vec![]));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_bool() {
            const QRY: &str = "{ bool }";
            const SUB: &str = "subscription { bool }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((graphql_value!({"bool": true}), vec![]));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_int() {
            const QRY: &str = "{ int }";
            const SUB: &str = "subscription { int }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((graphql_value!({"int": 42}), vec![]));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_float() {
            const QRY: &str = "{ float }";
            const SUB: &str = "subscription { float }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((graphql_value!({"float": 3.14}), vec![]));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_string() {
            const QRY: &str = "{ string }";
            const SUB: &str = "subscription { string }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((graphql_value!({"string": "Galadriel"}), vec![]));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_array() {
            const QRY: &str = "{ array }";
            const SUB: &str = "subscription { array }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((graphql_value!({"array": ["Ai", "Ambarendya!"]}), vec![]));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_object() {
            const QRY: &str = "{ object }";
            const SUB: &str = "subscription { object }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((
                graphql_value!({
                    "object": {"message": ["Ai", "Ambarendya!"]},
                }),
                vec![],
            ));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_nullable() {
            const QRY: &str = "{ nullable }";
            const SUB: &str = "subscription { nullable }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((
                graphql_value!({
                    "nullable": {"message": ["Ai", "Ambarendya!"]},
                }),
                vec![],
            ));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_fallible() {
            const QRY: &str = "{ fallible }";
            const SUB: &str = "subscription { fallible }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((
                graphql_value!({
                    "fallible": {"message": ["Ai", "Ambarendya!"]},
                }),
                vec![],
            ));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_fields() {
            const QRY: &str = r#"{
                object { message }
                nullable { message }
                fallible { message }
            }"#;
            const SUB1: &str = r#"subscription {
                object { message }
            }"#;
            const SUB2: &str = r#"subscription {
                nullable { message }
            }"#;
            const SUB3: &str = r#"subscription {
                fallible { message }
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((
                graphql_value!({
                    "object": {"message": ["Ai", "Ambarendya!"]},
                    "nullable": {"message": ["Ai", "Ambarendya!"]},
                    "fallible": {"message": ["Ai", "Ambarendya!"]},
                }),
                vec![],
            ));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB1, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "object": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                resolve_into_stream(SUB2, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "nullable": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                resolve_into_stream(SUB3, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "fallible": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
        }

        #[tokio::test]
        async fn resolves_unknown_fields_as_null() {
            const QRY: &str = r#"{
                object { message, friend }
                nullable { message, mellon }
                fallible { message, freund }
            }"#;
            const SUB1: &str = r#"subscription {
                object { message, friend }
            }"#;
            const SUB2: &str = r#"subscription {
                nullable { message, mellon }
            }"#;
            const SUB3: &str = r#"subscription {
                fallible { message, freund }
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((
                graphql_value!({
                    "object": {
                        "message": ["Ai", "Ambarendya!"],
                        "friend": null,
                    },
                    "nullable": {
                        "message": ["Ai", "Ambarendya!"],
                        "mellon": null,
                    },
                    "fallible": {
                        "message": ["Ai", "Ambarendya!"],
                        "freund": null,
                    },
                }),
                vec![],
            ));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB1, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "object": {
                            "message": ["Ai", "Ambarendya!"],
                            "friend": null,
                        },
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                resolve_into_stream(SUB2, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "nullable": {
                            "message": ["Ai", "Ambarendya!"],
                            "mellon": null,
                        },
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                resolve_into_stream(SUB3, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "fallible": {
                            "message": ["Ai", "Ambarendya!"],
                            "freund": null,
                        },
                    }),
                    vec![],
                )),
            );
        }

        #[tokio::test]
        async fn resolves_nested_object_fields() {
            const QRY: &str = "{ nested { message { body } } }";
            const SUB: &str = "subscription { nested { message { body } } }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((
                graphql_value!({
                    "nested": {"message": {"body": "Ambarendya!"}},
                }),
                vec![],
            ));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_nested_unknown_object_fields() {
            const QRY: &str = "{ nested { message { body, foo } } }";
            const SUB: &str = "subscription { nested { message { body, foo } } }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((
                graphql_value!({
                    "nested": {"message": {
                        "body": "Ambarendya!",
                        "foo": null,
                    }},
                }),
                vec![],
            ));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_nested_aliased_object_fields() {
            const QRY: &str = "{ nested { m: message { b: body } } }";
            const SUB: &str = "subscription { nested { m: message { b: body } } }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((
                graphql_value!({
                    "nested": {"m": {"b": "Ambarendya!"}},
                }),
                vec![],
            ));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn allows_fields_on_null() {
            const QRY: &str = "{ null { message } }";
            const SUB: &str = "subscription { null { message } }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((graphql_value!({ "null": null }), vec![]));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn errors_selecting_fields_on_leaf_value() {
            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok(Some("cannot select fields on a leaf opaque JSON value"));

            for qry in [
                "{ bool { message } }",
                "{ int { message } }",
                "{ float { message } }",
                "{ string { message } }",
                "{ array { message } }",
                "{ object { message { body } } }",
                "{ nested { message { body { theme } } } }",
            ] {
                let res = execute(qry, None, &schema, &graphql_vars! {}, &()).await;
                assert_eq!(
                    res.as_ref()
                        .map(|(_, errs)| errs.first().map(|e| e.error().message())),
                    expected,
                    "query: {}\nactual result: {:?}",
                    qry,
                    res,
                );

                let res = execute_sync(qry, None, &schema, &graphql_vars! {}, &());
                assert_eq!(
                    res.as_ref()
                        .map(|(_, errs)| errs.first().map(|e| e.error().message())),
                    expected,
                    "query: {}\nactual result: {:?}",
                    qry,
                    res,
                );

                let sub = format!("subscription {}", qry);
                let res = resolve_into_stream(&sub, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await;
                assert_eq!(
                    res.as_ref()
                        .map(|(_, errs)| errs.first().map(|e| e.error().message())),
                    expected,
                    "query: {}\nactual result: {:?}",
                    qry,
                    res,
                );
            }
        }

        #[tokio::test]
        async fn represents_scalar() {
            const QRY: &str = r#"{ __type(name: "Json") { kind } }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((graphql_value!({"__type": {"kind": "SCALAR"}}), vec![]));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
        }
    }

    mod as_input {
        use serde_json::Value;

        use crate::{
            execute, graphql_object, graphql_vars, EmptyMutation, EmptySubscription, RootNode,
        };

        struct Query;

        #[graphql_object]
        impl Query {
            fn input(arg: Value) -> Value {
                arg
            }
        }

        #[tokio::test]
        async fn accepts_null() {
            const DOC: &str = r#"{
                null: input(arg: null)
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({ "null": None }), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_bool() {
            const DOC: &str = r#"{
                bool: input(arg: true)
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"bool": true}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_int() {
            const DOC: &str = r#"{
                int: input(arg: 42)
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"int": 42}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_float() {
            const DOC: &str = r#"{
                float: input(arg: 3.14)
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"float": 3.14}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_string() {
            const DOC: &str = r#"{
                string: input(arg: "Galadriel")
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"string": "Galadriel"}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_array() {
            const DOC: &str = r#"{
                array: input(arg: ["Ai", "Ambarendya!"])
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"array": ["Ai", "Ambarendya!"]}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_object() {
            const DOC: &str = r#"{
                object: input(arg: {message: ["Ai", "Ambarendya!"]})
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((
                    graphql_value!({
                        "object": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
        }
    }
}

#[cfg(test)]
mod json_test {
    mod as_output {
        use futures::FutureExt as _;
        use serde::{Deserialize, Serialize};
        use serde_json::Value;

        use crate::{
            execute, execute_sync, graphql_object, graphql_subscription, graphql_vars,
            resolve_into_stream,
            tests::util::{extract_next, stream, Stream},
            EmptyMutation, FieldResult, RootNode,
        };

        use super::super::Json;

        #[derive(Debug, Deserialize, Serialize)]
        struct Message {
            message: Vec<String>,
        }

        #[derive(Debug, Deserialize, Serialize)]
        struct Envelope {
            envelope: Message,
        }

        struct Query;

        #[graphql_object]
        impl Query {
            fn null() -> Json {
                Value::Null.into()
            }

            fn bool() -> Json<bool> {
                true.into()
            }

            fn int() -> Json<i32> {
                42.into()
            }

            fn float() -> Json<f64> {
                3.14.into()
            }

            fn string() -> Json<String> {
                Json::wrap("Galadriel".into())
            }

            fn array() -> Json<Vec<String>> {
                vec!["Ai".into(), "Ambarendya!".into()].into()
            }

            fn object() -> Json<Message> {
                Json::wrap(Message {
                    message: vec!["Ai".into(), "Ambarendya!".into()],
                })
            }

            fn nullable() -> Option<Json<Message>> {
                Some(Json::wrap(Message {
                    message: vec!["Ai".into(), "Ambarendya!".into()],
                }))
            }

            fn fallible() -> FieldResult<Json<Message>> {
                Ok(Json::wrap(Message {
                    message: vec!["Ai".into(), "Ambarendya!".into()],
                }))
            }

            fn nested() -> Json<Envelope> {
                Json::wrap(Envelope {
                    envelope: Message {
                        message: vec!["Ai".into(), "Ambarendya!".into()],
                    },
                })
            }
        }

        struct Subscription;

        #[graphql_subscription]
        impl Subscription {
            async fn null() -> Stream<Json> {
                stream(Value::Null.into())
            }

            async fn bool() -> Stream<Json<bool>> {
                stream(true.into())
            }

            async fn int() -> Stream<Json<i32>> {
                stream(42.into())
            }

            async fn float() -> Stream<Json<f64>> {
                stream(3.14.into())
            }

            async fn string() -> Stream<Json<String>> {
                stream(Json::wrap("Galadriel".into()))
            }

            async fn array() -> Stream<Json<Vec<String>>> {
                stream(Json::wrap(vec!["Ai".into(), "Ambarendya!".into()]))
            }

            async fn object() -> Stream<Json<Message>> {
                stream(Json::wrap(Message {
                    message: vec!["Ai".into(), "Ambarendya!".into()],
                }))
            }

            async fn nullable() -> Stream<Option<Json<Message>>> {
                stream(Some(Json::wrap(Message {
                    message: vec!["Ai".into(), "Ambarendya!".into()],
                })))
            }

            async fn fallible() -> FieldResult<Stream<FieldResult<Json<Message>>>> {
                Ok(stream(Ok(Json::wrap(Message {
                    message: vec!["Ai".into(), "Ambarendya!".into()],
                }))))
            }

            async fn nested() -> Stream<Json<Envelope>> {
                stream(Json::wrap(Envelope {
                    envelope: Message {
                        message: vec!["Ai".into(), "Ambarendya!".into()],
                    },
                }))
            }
        }

        #[tokio::test]
        async fn resolves_null() {
            const QRY: &str = "{ null }";
            const SUB: &str = "subscription { null }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((graphql_value!({ "null": null }), vec![]));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_bool() {
            const QRY: &str = "{ bool }";
            const SUB: &str = "subscription { bool }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((graphql_value!({"bool": true}), vec![]));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_int() {
            const QRY: &str = "{ int }";
            const SUB: &str = "subscription { int }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((graphql_value!({"int": 42}), vec![]));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_float() {
            const QRY: &str = "{ float }";
            const SUB: &str = "subscription { float }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((graphql_value!({"float": 3.14}), vec![]));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_string() {
            const QRY: &str = "{ string }";
            const SUB: &str = "subscription { string }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((graphql_value!({"string": "Galadriel"}), vec![]));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_array() {
            const QRY: &str = "{ array }";
            const SUB: &str = "subscription { array }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((graphql_value!({"array": ["Ai", "Ambarendya!"]}), vec![]));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_object() {
            const QRY: &str = "{ object }";
            const SUB: &str = "subscription { object }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((
                graphql_value!({
                    "object": {"message": ["Ai", "Ambarendya!"]},
                }),
                vec![],
            ));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_nullable() {
            const QRY: &str = "{ nullable }";
            const SUB: &str = "subscription { nullable }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((
                graphql_value!({
                    "nullable": {"message": ["Ai", "Ambarendya!"]},
                }),
                vec![],
            ));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_fallible() {
            const QRY: &str = "{ fallible }";
            const SUB: &str = "subscription { fallible }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((
                graphql_value!({
                    "fallible": {"message": ["Ai", "Ambarendya!"]},
                }),
                vec![],
            ));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_fields() {
            const QRY: &str = r#"{
                object { message }
                nullable { message }
                fallible { message }
            }"#;
            const SUB1: &str = r#"subscription {
                object { message }
            }"#;
            const SUB2: &str = r#"subscription {
                nullable { message }
            }"#;
            const SUB3: &str = r#"subscription {
                fallible { message }
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((
                graphql_value!({
                    "object": {"message": ["Ai", "Ambarendya!"]},
                    "nullable": {"message": ["Ai", "Ambarendya!"]},
                    "fallible": {"message": ["Ai", "Ambarendya!"]},
                }),
                vec![],
            ));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB1, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "object": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                resolve_into_stream(SUB2, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "nullable": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                resolve_into_stream(SUB3, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "fallible": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
        }

        #[tokio::test]
        async fn resolves_unknown_fields_as_null() {
            const QRY: &str = r#"{
                object { message, friend }
                nullable { message, mellon }
                fallible { message, freund }
            }"#;
            const SUB1: &str = r#"subscription {
                object { message, friend }
            }"#;
            const SUB2: &str = r#"subscription {
                nullable { message, mellon }
            }"#;
            const SUB3: &str = r#"subscription {
                fallible { message, freund }
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((
                graphql_value!({
                    "object": {
                        "message": ["Ai", "Ambarendya!"],
                        "friend": null,
                    },
                    "nullable": {
                        "message": ["Ai", "Ambarendya!"],
                        "mellon": null,
                    },
                    "fallible": {
                        "message": ["Ai", "Ambarendya!"],
                        "freund": null,
                    },
                }),
                vec![],
            ));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB1, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "object": {
                            "message": ["Ai", "Ambarendya!"],
                            "friend": null,
                        },
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                resolve_into_stream(SUB2, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "nullable": {
                            "message": ["Ai", "Ambarendya!"],
                            "mellon": null,
                        },
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                resolve_into_stream(SUB3, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "fallible": {
                            "message": ["Ai", "Ambarendya!"],
                            "freund": null,
                        },
                    }),
                    vec![],
                )),
            );
        }

        #[tokio::test]
        async fn resolves_nested_object_fields() {
            const QRY: &str = "{ nested { envelope { message } } }";
            const SUB: &str = "subscription { nested { envelope { message } } }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((
                graphql_value!({
                    "nested": {"envelope": {"message": ["Ai", "Ambarendya!"]}},
                }),
                vec![],
            ));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_nested_unknown_object_fields() {
            const QRY: &str = "{ nested { envelope { message, foo } } }";
            const SUB: &str = "subscription { nested { envelope { message, foo } } }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((
                graphql_value!({
                    "nested": {"envelope": {
                        "message": ["Ai", "Ambarendya!"],
                        "foo": null,
                    }},
                }),
                vec![],
            ));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn resolves_nested_aliased_object_fields() {
            const QRY: &str = "{ nested { e: envelope { m: message } } }";
            const SUB: &str = "subscription { nested { e: envelope { m: message } } }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((
                graphql_value!({
                    "nested": {"e": {"m": ["Ai", "Ambarendya!"]}},
                }),
                vec![],
            ));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn allows_fields_on_null() {
            const QRY: &str = "{ null { message } }";
            const SUB: &str = "subscription { null { message } }";

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((graphql_value!({ "null": null }), vec![]));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                expected,
            );
        }

        #[tokio::test]
        async fn errors_selecting_fields_on_leaf_value() {
            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok(Some("cannot select fields on a leaf opaque JSON value"));

            for qry in [
                "{ bool { message } }",
                "{ int { message } }",
                "{ float { message } }",
                "{ string { message } }",
                "{ array { message } }",
                "{ object { message { body } } }",
                "{ nested { envelope { message { theme } } } }",
            ] {
                let res = execute(qry, None, &schema, &graphql_vars! {}, &()).await;
                assert_eq!(
                    res.as_ref()
                        .map(|(_, errs)| errs.first().map(|e| e.error().message())),
                    expected,
                    "query: {}\nactual result: {:?}",
                    qry,
                    res,
                );

                let res = execute_sync(qry, None, &schema, &graphql_vars! {}, &());
                assert_eq!(
                    res.as_ref()
                        .map(|(_, errs)| errs.first().map(|e| e.error().message())),
                    expected,
                    "query: {}\nactual result: {:?}",
                    qry,
                    res,
                );

                let sub = format!("subscription {}", qry);
                let res = resolve_into_stream(&sub, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await;
                assert_eq!(
                    res.as_ref()
                        .map(|(_, errs)| errs.first().map(|e| e.error().message())),
                    expected,
                    "query: {}\nactual result: {:?}",
                    qry,
                    res,
                );
            }
        }

        #[tokio::test]
        async fn represents_scalar() {
            const QRY: &str = r#"{ __type(name: "Json") { kind } }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);

            let expected = Ok((graphql_value!({"__type": {"kind": "SCALAR"}}), vec![]));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
        }
    }

    mod as_input {
        use serde::{Deserialize, Serialize};

        use crate::{
            execute, graphql_object, graphql_vars, EmptyMutation, EmptySubscription, RootNode,
        };

        use super::super::Json;

        #[derive(Debug, Deserialize, Serialize)]
        struct Message {
            message: Vec<String>,
        }

        #[derive(Debug, Deserialize, Serialize)]
        struct Envelope {
            envelope: Message,
        }

        struct Query;

        #[graphql_object]
        impl Query {
            fn any(arg: Json) -> Json {
                arg
            }

            fn bool(arg: Json<bool>) -> Json<bool> {
                arg
            }

            fn int(arg: Json<i32>) -> Json<i32> {
                arg
            }

            fn float(arg: Json<f64>) -> Json<f64> {
                arg
            }

            fn string(arg: Json<String>) -> Json<String> {
                arg
            }

            fn array(arg: Json<Vec<String>>) -> Json<Vec<String>> {
                arg
            }

            fn object(arg: Json<Envelope>) -> Json<Envelope> {
                arg
            }
        }

        #[tokio::test]
        async fn accepts_null() {
            const DOC: &str = r#"{
                null: any(arg: null)
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({ "null": None }), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_bool() {
            const DOC: &str = r#"{
                bool(arg: true)
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"bool": true}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_int() {
            const DOC: &str = r#"{
                int(arg: 42)
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"int": 42}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_float() {
            const DOC: &str = r#"{
                float(arg: 3.14)
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"float": 3.14}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_string() {
            const DOC: &str = r#"{
                string(arg: "Galadriel")
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"string": "Galadriel"}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_array() {
            const DOC: &str = r#"{
                array(arg: ["Ai", "Ambarendya!"])
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"array": ["Ai", "Ambarendya!"]}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_object() {
            const DOC: &str = r#"{
                object(arg: {envelope: {message: ["Ai", "Ambarendya!"]}})
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((
                    graphql_value!({
                        "object": {"envelope": {"message": ["Ai", "Ambarendya!"]}},
                    }),
                    vec![],
                )),
            );
        }

        #[tokio::test]
        async fn errors_on_invalid_object() {
            const DOC: &str = r#"{
                object(arg: {envelope: ["Ai", "Ambarendya!"]})
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

            let res = execute(DOC, None, &schema, &graphql_vars! {}, &()).await;
            assert_eq!(
                res.as_ref()
                    .map(|(_, errs)| errs.first().map(|e| e.error().message())),
                Ok(Some(r#"invalid type: string "Ai", expected a sequence"#)),
            );
        }
    }
}

#[cfg(test)]
mod typed_test {
    mod as_output {
        use serde_json::{json, Value};

        use crate::{
            execute, execute_sync, graphql_vars, DefaultScalarValue, EmptyMutation,
            EmptySubscription, RootNode,
        };

        use super::super::{Info, Typed};

        const SDL: &str = r#"
            type Bar {
                location: String
                capacity: Int
                open: Boolean!
                rating: Float
                foo: Foo
            }
            type Foo {
                id: [ID]
                message: String
                bar: Bar
            }
        "#;

        #[tokio::test]
        async fn resolves() {
            const QRY: &str = r#"{
                id
                message
                bar {
                    location
                    capacity
                    open
                    rating
                    foo {
                        message
                    }
                }
            }"#;

            let data = Typed::wrap(json!({
                "id": ["foo-1"],
                "message": "hello world",
                "bar": {
                    "location": "downtown",
                    "capacity": 80,
                    "open": true,
                    "rating": 4.5,
                    "foo": {
                        "message": "drink more",
                    },
                },
            }));
            let schema = <RootNode<_, _, _, DefaultScalarValue>>::new_with_info(
                data.clone(),
                EmptyMutation::new(),
                EmptySubscription::new(),
                Info::parse("Foo", SDL).unwrap(),
                (),
                (),
            );

            let expected = Ok((data.into_inner().into(), vec![]));

            assert_eq!(
                execute(QRY, None, &schema, &graphql_vars! {}, &()).await,
                expected,
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &graphql_vars! {}, &()),
                expected,
            );
        }
    }
}

//------------------------------------------------------------------------------

/*

impl<S: ScalarValue> GraphQLValue<S> for Json {
    type Context = ();
    type TypeInfo = TypeInfo;

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        Some(info.name.as_str())
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        selection: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        if let Some(sel) = selection {
            // resolve this value as an object
            let mut res = juniper::Object::with_capacity(sel.len());
            Ok(
                if resolve_selection_set_into(self, info, sel, executor, &mut res) {
                    Value::Object(res)
                } else {
                    Value::null()
                },
            )
        } else {
            // resolve this value as leaf
            match self {
                Json::Null => Ok(Value::null()),
                Json::Bool(value) => executor.resolve::<bool>(&(), value),
                Json::Number(value) => {
                    if value.is_f64() {
                        executor.resolve::<f64>(&(), &value.as_f64().unwrap())
                    } else if value.is_i64() {
                        executor.resolve::<i32>(&(), &(value.as_i64().unwrap() as i32))
                    } else if value.is_u64() {
                        executor.resolve::<i32>(&(), &(value.as_u64().unwrap() as i32))
                    } else {
                        panic!("invalid number")
                    }
                }
                Json::String(value) => executor.resolve::<String>(&(), value),
                _ => Err(FieldError::new("not a leaf value", Value::Null)),
            }
        }
    }

    fn resolve_field(
        &self,
        _info: &Self::TypeInfo,
        field_name: &str,
        _: &Arguments<S>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        match self {
            Json::Object(fields) => {
                let field_value = fields.get(field_name);
                match field_value {
                    None => Ok(Value::null()),
                    Some(field_value) => {
                        let current_type = executor.current_type();
                        let field_info = &TypeInfo {
                            schema: None,
                            name: current_type
                                .innermost_concrete()
                                .name()
                                .unwrap()
                                .to_string(),
                        };
                        if current_type.list_contents().is_some() {
                            match field_value {
                                Json::Null => Ok(Value::null()),
                                Json::Array(field_value) => {
                                    executor.resolve::<Vec<Json>>(field_info, field_value)
                                }
                                _ => Err(FieldError::new("not an array", Value::Null)),
                            }
                        } else {
                            executor.resolve::<Json>(field_info, &field_value)
                        }
                    }
                }
            }
            _ => Err(FieldError::new("not an object value", Value::Null)),
        }
    }
}
*/
/*
#[cfg(test)]
mod tests {
    use juniper::{
        execute_sync, graphql_object, graphql_value,
        integrations::serde_json::{TypeInfo, TypedJson, TypedJsonInfo},
        EmptyMutation, EmptySubscription, FieldResult, RootNode, ToInputValue, Variables,
    };
    use serde_json::json;

    #[test]
    fn sdl_type_info() {
        let sdl = r#"
            type Bar {
                location: String
                capacity: Int
                open: Boolean!
                rating: Float
                foo: Foo
            }
            type Foo {
                message: String
                bar: Bar
            }
        "#;

        let info = TypeInfo {
            name: "Foo".to_string(),
            schema: Some(sdl.to_string()),
        };

        let data = json!({
            "message": "hello world",
            "bar": {
                    "location": "downtown",
                    "capacity": 80,
                    "open": true,
                    "rating": 4.5,
                    "foo": {
                        "message": "drink more"
                    }
                },
        });

        let schema = <RootNode<_, _, _>>::new_with_info(
            data,
            EmptyMutation::new(),
            EmptySubscription::new(),
            info,
            (),
            (),
        );

        // print!("{}", schema.as_schema_language());

        let query = r#"{
            message
            bar {
                location
                capacity
                open
                rating
                foo {
                    message
                }
            }
        }"#;

        assert_eq!(
            execute_sync(query, None, &schema, &graphql_vars! {}, &()),
            Ok((
                graphql_value!({
                    "message": "hello world",
                    "bar": {
                        "location": "downtown",
                        "capacity": 80,
                        "open": true,
                        "rating": 4.5,
                        "foo": {
                            "message": "drink more"
                        }
                    }
                }),
                vec![],
            )),
        );
    }

    #[test]
    fn required_field() {
        let sdl = r#"
            type Bar {
                location: String
                open: Boolean!
            }
            type Foo {
                message: String
                bar: Bar
            }
        "#;

        let info = TypeInfo {
            name: "Foo".to_string(),
            schema: Some(sdl.to_string()),
        };

        let data = json!({
            "message": "hello world",
            "bar": {
                "capacity": 80,
            },
        });

        let schema = <RootNode<_, _, _>>::new_with_info(
            data,
            EmptyMutation::new(),
            EmptySubscription::new(),
            info,
            (),
            (),
        );

        let query = r#"{
            message
            bar {
                location
                open
            }
        }"#;

        assert_eq!(
            execute_sync(query, None, &schema, &graphql_vars! {}, &()),
            Ok((
                graphql_value!({
                    "message": "hello world",
                    "bar": None,
                }),
                vec![],
            )),
        );
    }

    #[test]
    fn array_field() {
        let sdl = r#"
            type Bar {
                location: [String]
                open: [Boolean!]
            }
            type Foo {
                message: [String]
                bar: [Bar]
            }
        "#;

        let info = TypeInfo {
            name: "Foo".to_string(),
            schema: Some(sdl.to_string()),
        };

        let data = json!({
            "message": ["hello world"],
            "bar": [{
                "location": ["Tampa"],
            }],
        });

        let schema: RootNode<_, _, _> = RootNode::new_with_info(
            data,
            EmptyMutation::new(),
            EmptySubscription::new(),
            info,
            (),
            (),
        );

        // print!("{}", schema.as_schema_language());

        let query = r#"{
            message
            bar {
                location
            }
        }"#;

        assert_eq!(
            execute_sync(query, None, &schema, &graphql_vars! {}, &()),
            Ok((
                graphql_value!({
                    "message": ["hello world"],
                    "bar": [{
                        "location": ["Tampa"],
                    }],
                }),
                vec![],
            )),
        );
    }

    #[test]
    fn test_as_field_of_output_type() {
        // We need a Foo wrapper associate a static SDL to the Foo type which
        struct Foo;
        impl TypedJsonInfo for Foo {
            fn type_name() -> &'static str {
                "Foo"
            }
            fn schema() -> &'static str {
                r#"
                type Foo {
                    message: [String]
                }
                "#
            }
        }

        struct Query;
        #[graphql_object]
        impl Query {
            fn foo() -> FieldResult<TypedJson<Foo>> {
                let data = json!({"message": ["Hello", "World"] });
                Ok(TypedJson::new(data))
            }
        }
        let schema = juniper::RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
        // Run the executor.
        let (res, _errors) = juniper::execute_sync(
            "query { foo { message } }",
            None,
            &schema,
            &graphql_vars! {},
            &(),
        )
        .unwrap();

        // Ensure the value matches.
        assert_eq!(
            res,
            graphql_value!({
                "foo": {"message":["Hello", "World"]},
            })
        );
    }

    #[test]
    fn test_as_field_of_input_type() {
        #[derive(Debug, Clone, PartialEq)]
        struct Foo;
        impl TypedJsonInfo for Foo {
            fn type_name() -> &'static str {
                "Foo"
            }
            fn schema() -> &'static str {
                r#"
                input Foo {
                    message: [String]
                }
                "#
            }
        }

        struct Query;
        #[graphql_object()]
        impl Query {
            fn foo(value: TypedJson<Foo>) -> FieldResult<bool> {
                Ok(value == TypedJson::new(json!({"message":["Hello", "World"]})))
            }
        }
        let schema = juniper::RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

        let vars = vec![(
            "value".to_owned(),
            graphql_value!({
                "message":["Hello", "World"],
            })
            .to_input_value(),
        )]
        .into_iter()
        .collect();

        // Run the executor.
        let (res, _errors) = juniper::execute_sync(
            "query example($value:Foo!){ foo(value: $value) }",
            None,
            &schema,
            &vars,
            &(),
        )
        .unwrap();

        // Ensure the value matches.
        assert_eq!(
            res,
            graphql_value!({
                "foo": true,
            }),
        );
    }
}
*/
