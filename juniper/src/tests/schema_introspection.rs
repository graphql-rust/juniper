use crate::value::{
    self,
    Value::{self, Null},
};

// Sort a nested schema Value.
// In particular, lists are sorted by the "name" key of children, if present.
// Only needed for comparisons.
pub(super) fn sort_schema_value(value: &mut Value) {
    match value {
        Value::Null | Value::Scalar(_) => {}
        Value::List(ref mut items) => {
            items.sort_by(|a, b| {
                let name_a = a
                    .as_object_value()
                    .and_then(|v| v.get_field_value("name"))
                    .and_then(|v| v.as_scalar_value::<String>())
                    .map(|x| x.as_str())
                    .unwrap_or("");
                let name_b = b
                    .as_object_value()
                    .and_then(|v| v.get_field_value("name"))
                    .and_then(|v| v.as_scalar_value::<String>())
                    .map(|x| x.as_str())
                    .unwrap_or("");
                name_a.cmp(name_b)
            });
            for item in items.iter_mut() {
                sort_schema_value(item);
            }
        }
        Value::Object(ref mut obj) => {
            obj.iter_mut()
                .for_each(|(_key, item)| sort_schema_value(item));
        }
    }
}

pub(crate) fn schema_introspection_result() -> value::Value {
    let mut v = graphql_value!({
        "__schema": {
          "queryType": {
            "name": "Query"
          },
          "mutationType": Null,
          "subscriptionType": Null,
          "types": [
            {
              "kind": "OBJECT",
              "name": "Human",
              "description": "A humanoid creature in the Star Wars universe.",
              "fields": [
                {
                  "name": "id",
                  "description": "The id of the human",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "name",
                  "description": "The name of the human",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "friends",
                  "description": "The friends of the human",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "INTERFACE",
                          "name": "Character",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "appearsIn",
                  "description": "Which movies they appear in",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "Episode",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "homePlanet",
                  "description": "The home planet of the human",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "inputFields": Null,
              "interfaces": [
                {
                  "kind": "INTERFACE",
                  "name": "Character",
                  "ofType": Null
                }
              ],
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "SCALAR",
              "name": "Boolean",
              "description": Null,
              "fields": Null,
              "inputFields": Null,
              "interfaces": Null,
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "OBJECT",
              "name": "__InputValue",
              "description": Null,
              "fields": [
                {
                  "name": "name",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "description",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "type",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "OBJECT",
                      "name": "__Type",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "defaultValue",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "inputFields": Null,
              "interfaces": [],
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "SCALAR",
              "name": "String",
              "description": Null,
              "fields": Null,
              "inputFields": Null,
              "interfaces": Null,
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "OBJECT",
              "name": "__Field",
              "description": Null,
              "fields": [
                {
                  "name": "name",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "description",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "args",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__InputValue",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "type",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "OBJECT",
                      "name": "__Type",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "isDeprecated",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "deprecationReason",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "inputFields": Null,
              "interfaces": [],
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "ENUM",
              "name": "__TypeKind",
              "description": "GraphQL type kind\n\nThe GraphQL specification defines a number of type kinds - the meta type of a type.",
              "fields": Null,
              "inputFields": Null,
              "interfaces": Null,
              "enumValues": [
                {
                  "name": "SCALAR",
                  "description": "## Scalar types\n\nScalar types appear as the leaf nodes of GraphQL queries. Strings, numbers, and booleans are the built in types, and while it's possible to define your own, it's relatively uncommon.",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "OBJECT",
                  "description": "## Object types\n\nThe most common type to be implemented by users. Objects have fields and can implement interfaces.",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "INTERFACE",
                  "description": "## Interface types\n\nInterface types are used to represent overlapping fields between multiple types, and can be queried for their concrete type.",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "UNION",
                  "description": "## Union types\n\nUnions are similar to interfaces but can not contain any fields on their own.",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "ENUM",
                  "description": "## Enum types\n\nLike scalars, enum types appear as the leaf nodes of GraphQL queries.",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "INPUT_OBJECT",
                  "description": "## Input objects\n\nRepresents complex values provided in queries _into_ the system.",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "LIST",
                  "description": "## List types\n\nRepresent lists of other types. This library provides implementations for vectors and slices, but other Rust types can be extended to serve as GraphQL lists.",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "NON_NULL",
                  "description": "## Non-null types\n\nIn GraphQL, nullable types are the default. By putting a `!` after a type, it becomes non-nullable.",
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "possibleTypes": Null
            },
            {
              "kind": "OBJECT",
              "name": "__Type",
              "description": Null,
              "fields": [
                {
                  "name": "name",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "description",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "kind",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "ENUM",
                      "name": "__TypeKind",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "fields",
                  "description": Null,
                  "args": [
                    {
                      "name": "includeDeprecated",
                      "description": Null,
                      "type": {
                        "kind": "SCALAR",
                        "name": "Boolean",
                        "ofType": Null
                      },
                      "defaultValue": "false"
                    }
                  ],
                  "type": {
                    "kind": "LIST",
                    "name": Null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": Null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__Field",
                        "ofType": Null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "ofType",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "OBJECT",
                    "name": "__Type",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "inputFields",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "LIST",
                    "name": Null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": Null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__InputValue",
                        "ofType": Null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "interfaces",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "LIST",
                    "name": Null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": Null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__Type",
                        "ofType": Null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "possibleTypes",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "LIST",
                    "name": Null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": Null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__Type",
                        "ofType": Null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "enumValues",
                  "description": Null,
                  "args": [
                    {
                      "name": "includeDeprecated",
                      "description": Null,
                      "type": {
                        "kind": "SCALAR",
                        "name": "Boolean",
                        "ofType": Null
                      },
                      "defaultValue": "false"
                    }
                  ],
                  "type": {
                    "kind": "LIST",
                    "name": Null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": Null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__EnumValue",
                        "ofType": Null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "inputFields": Null,
              "interfaces": [],
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "OBJECT",
              "name": "__Schema",
              "description": Null,
              "fields": [
                {
                  "name": "types",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__Type",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "queryType",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "OBJECT",
                      "name": "__Type",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "mutationType",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "OBJECT",
                    "name": "__Type",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "subscriptionType",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "OBJECT",
                    "name": "__Type",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "directives",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__Directive",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "inputFields": Null,
              "interfaces": [],
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "OBJECT",
              "name": "Droid",
              "description": "A mechanical creature in the Star Wars universe.",
              "fields": [
                {
                  "name": "id",
                  "description": "The id of the droid",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "name",
                  "description": "The name of the droid",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "friends",
                  "description": "The friends of the droid",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "INTERFACE",
                          "name": "Character",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "appearsIn",
                  "description": "Which movies they appear in",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "Episode",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "primaryFunction",
                  "description": "The primary function of the droid",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "inputFields": Null,
              "interfaces": [
                {
                  "kind": "INTERFACE",
                  "name": "Character",
                  "ofType": Null
                }
              ],
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "OBJECT",
              "name": "Query",
              "description": "The root query object of the schema",
              "fields": [
                {
                  "name": "human",
                  "description": Null,
                  "args": [
                    {
                      "name": "id",
                      "description": "id of the human",
                      "type": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "SCALAR",
                          "name": "String",
                          "ofType": Null
                        }
                      },
                      "defaultValue": Null
                    }
                  ],
                  "type": {
                    "kind": "OBJECT",
                    "name": "Human",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "droid",
                  "description": Null,
                  "args": [
                    {
                      "name": "id",
                      "description": Null,
                      "description": "id of the droid",
                      "type": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "SCALAR",
                          "name": "String",
                          "ofType": Null
                        }
                      },
                      "defaultValue": Null
                    }
                  ],
                  "type": {
                    "kind": "OBJECT",
                    "name": "Droid",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "hero",
                  "description": Null,
                  "args": [
                    {
                      "name": "episode",
                      "description": "If omitted, returns the hero of the whole saga. If provided, returns the hero of that particular episode",
                      "type": {
                        "kind": "ENUM",
                        "name": "Episode",
                        "ofType": Null
                      },
                      "defaultValue": Null
                    }
                  ],
                  "type": {
                    "kind": "INTERFACE",
                    "name": "Character",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "inputFields": Null,
              "interfaces": [],
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "OBJECT",
              "name": "__EnumValue",
              "description": Null,
              "fields": [
                {
                  "name": "name",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "description",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "isDeprecated",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "deprecationReason",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "inputFields": Null,
              "interfaces": [],
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "ENUM",
              "name": "Episode",
              "description": Null,
              "fields": Null,
              "inputFields": Null,
              "interfaces": Null,
              "enumValues": [
                {
                  "name": "NEW_HOPE",
                  "description": Null,
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "EMPIRE",
                  "description": Null,
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "JEDI",
                  "description": Null,
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "possibleTypes": Null
            },
            {
              "kind": "ENUM",
              "name": "__DirectiveLocation",
              "description": Null,
              "fields": Null,
              "inputFields": Null,
              "interfaces": Null,
              "enumValues": [
                {
                  "name": "QUERY",
                  "description": Null,
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "MUTATION",
                  "description": Null,
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "SUBSCRIPTION",
                  "description": Null,
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "FIELD",
                  "description": Null,
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "FRAGMENT_DEFINITION",
                  "description": Null,
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "FRAGMENT_SPREAD",
                  "description": Null,
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "INLINE_FRAGMENT",
                  "description": Null,
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "possibleTypes": Null
            },
            {
              "kind": "INTERFACE",
              "name": "Character",
              "description": "A character in the Star Wars Trilogy",
              "fields": [
                {
                  "name": "id",
                  "description": "The id of the character",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "name",
                  "description": "The name of the character",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "friends",
                  "description": "The friends of the character",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "INTERFACE",
                          "name": "Character",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "appearsIn",
                  "description": "Which movies they appear in",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "Episode",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "inputFields": Null,
              "interfaces": Null,
              "enumValues": Null,
              "possibleTypes": [
                {
                  "kind": "OBJECT",
                  "name": "Human",
                  "ofType": Null
                },
                {
                  "kind": "OBJECT",
                  "name": "Droid",
                  "ofType": Null
                }
              ]
            },
            {
              "kind": "OBJECT",
              "name": "__Directive",
              "description": Null,
              "fields": [
                {
                  "name": "name",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "description",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "locations",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "__DirectiveLocation",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "args",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__InputValue",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "onOperation",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": true,
                  "deprecationReason": "Use the locations array instead"
                },
                {
                  "name": "onFragment",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": true,
                  "deprecationReason": "Use the locations array instead"
                },
                {
                  "name": "onField",
                  "description": Null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": true,
                  "deprecationReason": "Use the locations array instead"
                }
              ],
              "inputFields": Null,
              "interfaces": [],
              "enumValues": Null,
              "possibleTypes": Null
            }
          ],
          "directives": [
            {
              "name": "skip",
              "description": Null,
              "locations": [
                "FIELD",
                "FRAGMENT_SPREAD",
                "INLINE_FRAGMENT"
              ],
              "args": [
                {
                  "name": "if",
                  "description": Null,
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": Null
                    }
                  },
                  "defaultValue": Null
                }
              ]
            },
            {
              "name": "include",
              "description": Null,
              "locations": [
                "FIELD",
                "FRAGMENT_SPREAD",
                "INLINE_FRAGMENT"
              ],
              "args": [
                {
                  "name": "if",
                  "description": Null,
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": Null
                    }
                  },
                  "defaultValue": Null
                }
              ]
            }
          ]
        }
    });
    sort_schema_value(&mut v);
    v
}

pub(crate) fn schema_introspection_result_without_descriptions() -> value::Value {
    let mut v = graphql_value!({
        "__schema": {
          "queryType": {
            "name": "Query"
          },
          "mutationType": Null,
          "subscriptionType": Null,
          "types": [
            {
              "kind": "OBJECT",
              "name": "Human",
              "fields": [
                {
                  "name": "id",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "name",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "friends",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "INTERFACE",
                          "name": "Character",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "appearsIn",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "Episode",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "homePlanet",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "inputFields": Null,
              "interfaces": [
                {
                  "kind": "INTERFACE",
                  "name": "Character",
                  "ofType": Null
                }
              ],
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "SCALAR",
              "name": "Boolean",
              "fields": Null,
              "inputFields": Null,
              "interfaces": Null,
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "OBJECT",
              "name": "__InputValue",
              "fields": [
                {
                  "name": "name",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "description",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "type",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "OBJECT",
                      "name": "__Type",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "defaultValue",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "inputFields": Null,
              "interfaces": [],
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "SCALAR",
              "name": "String",
              "fields": Null,
              "inputFields": Null,
              "interfaces": Null,
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "OBJECT",
              "name": "__Field",
              "fields": [
                {
                  "name": "name",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "description",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "args",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__InputValue",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "type",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "OBJECT",
                      "name": "__Type",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "isDeprecated",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "deprecationReason",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "inputFields": Null,
              "interfaces": [],
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "ENUM",
              "name": "__TypeKind",
              "fields": Null,
              "inputFields": Null,
              "interfaces": Null,
              "enumValues": [
                {
                  "name": "SCALAR",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "OBJECT",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "INTERFACE",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "UNION",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "ENUM",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "INPUT_OBJECT",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "LIST",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "NON_NULL",
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "possibleTypes": Null
            },
            {
              "kind": "OBJECT",
              "name": "__Type",
              "fields": [
                {
                  "name": "name",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "description",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "kind",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "ENUM",
                      "name": "__TypeKind",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "fields",
                  "args": [
                    {
                      "name": "includeDeprecated",
                      "type": {
                        "kind": "SCALAR",
                        "name": "Boolean",
                        "ofType": Null
                      },
                      "defaultValue": "false"
                    }
                  ],
                  "type": {
                    "kind": "LIST",
                    "name": Null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": Null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__Field",
                        "ofType": Null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "ofType",
                  "args": [],
                  "type": {
                    "kind": "OBJECT",
                    "name": "__Type",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "inputFields",
                  "args": [],
                  "type": {
                    "kind": "LIST",
                    "name": Null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": Null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__InputValue",
                        "ofType": Null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "interfaces",
                  "args": [],
                  "type": {
                    "kind": "LIST",
                    "name": Null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": Null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__Type",
                        "ofType": Null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "possibleTypes",
                  "args": [],
                  "type": {
                    "kind": "LIST",
                    "name": Null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": Null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__Type",
                        "ofType": Null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "enumValues",
                  "args": [
                    {
                      "name": "includeDeprecated",
                      "type": {
                        "kind": "SCALAR",
                        "name": "Boolean",
                        "ofType": Null
                      },
                      "defaultValue": "false"
                    }
                  ],
                  "type": {
                    "kind": "LIST",
                    "name": Null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": Null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__EnumValue",
                        "ofType": Null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "inputFields": Null,
              "interfaces": [],
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "OBJECT",
              "name": "__Schema",
              "fields": [
                {
                  "name": "types",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__Type",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "queryType",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "OBJECT",
                      "name": "__Type",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "mutationType",
                  "args": [],
                  "type": {
                    "kind": "OBJECT",
                    "name": "__Type",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "subscriptionType",
                  "args": [],
                  "type": {
                    "kind": "OBJECT",
                    "name": "__Type",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "directives",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__Directive",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "inputFields": Null,
              "interfaces": [],
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "OBJECT",
              "name": "Droid",
              "fields": [
                {
                  "name": "id",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "name",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "friends",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "INTERFACE",
                          "name": "Character",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "appearsIn",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "Episode",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "primaryFunction",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "inputFields": Null,
              "interfaces": [
                {
                  "kind": "INTERFACE",
                  "name": "Character",
                  "ofType": Null
                }
              ],
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "OBJECT",
              "name": "Query",
              "fields": [
                {
                  "name": "human",
                  "args": [
                    {
                      "name": "id",
                      "type": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "SCALAR",
                          "name": "String",
                          "ofType": Null
                        }
                      },
                      "defaultValue": Null
                    }
                  ],
                  "type": {
                    "kind": "OBJECT",
                    "name": "Human",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "droid",
                  "args": [
                    {
                      "name": "id",
                      "type": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "SCALAR",
                          "name": "String",
                          "ofType": Null
                        }
                      },
                      "defaultValue": Null
                    }
                  ],
                  "type": {
                    "kind": "OBJECT",
                    "name": "Droid",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "hero",
                  "args": [
                    {
                      "name": "episode",
                      "type": {
                        "kind": "ENUM",
                        "name": "Episode",
                        "ofType": Null
                      },
                      "defaultValue": Null
                    }
                  ],
                  "type": {
                    "kind": "INTERFACE",
                    "name": "Character",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "inputFields": Null,
              "interfaces": [],
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "OBJECT",
              "name": "__EnumValue",
              "fields": [
                {
                  "name": "name",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "description",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "isDeprecated",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "deprecationReason",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "inputFields": Null,
              "interfaces": [],
              "enumValues": Null,
              "possibleTypes": Null
            },
            {
              "kind": "ENUM",
              "name": "Episode",
              "fields": Null,
              "inputFields": Null,
              "interfaces": Null,
              "enumValues": [
                {
                  "name": "NEW_HOPE",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "EMPIRE",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "JEDI",
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "possibleTypes": Null
            },
            {
              "kind": "ENUM",
              "name": "__DirectiveLocation",
              "fields": Null,
              "inputFields": Null,
              "interfaces": Null,
              "enumValues": [
                {
                  "name": "QUERY",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "MUTATION",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "SUBSCRIPTION",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "FIELD",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "FRAGMENT_DEFINITION",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "FRAGMENT_SPREAD",
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "INLINE_FRAGMENT",
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "possibleTypes": Null
            },
            {
              "kind": "INTERFACE",
              "name": "Character",
              "fields": [
                {
                  "name": "id",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "name",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "friends",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "INTERFACE",
                          "name": "Character",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "appearsIn",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "Episode",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                }
              ],
              "inputFields": Null,
              "interfaces": Null,
              "enumValues": Null,
              "possibleTypes": [
                {
                  "kind": "OBJECT",
                  "name": "Human",
                  "ofType": Null
                },
                {
                  "kind": "OBJECT",
                  "name": "Droid",
                  "ofType": Null
                }
              ]
            },
            {
              "kind": "OBJECT",
              "name": "__Directive",
              "fields": [
                {
                  "name": "name",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "description",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": Null
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "locations",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "__DirectiveLocation",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "args",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "LIST",
                      "name": Null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": Null,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__InputValue",
                          "ofType": Null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": Null
                },
                {
                  "name": "onOperation",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": true,
                  "deprecationReason": "Use the locations array instead"
                },
                {
                  "name": "onFragment",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": true,
                  "deprecationReason": "Use the locations array instead"
                },
                {
                  "name": "onField",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": Null
                    }
                  },
                  "isDeprecated": true,
                  "deprecationReason": "Use the locations array instead"
                }
              ],
              "inputFields": Null,
              "interfaces": [],
              "enumValues": Null,
              "possibleTypes": Null
            }
          ],
          "directives": [
            {
              "name": "skip",
              "locations": [
                "FIELD",
                "FRAGMENT_SPREAD",
                "INLINE_FRAGMENT"
              ],
              "args": [
                {
                  "name": "if",
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": Null
                    }
                  },
                  "defaultValue": Null
                }
              ]
            },
            {
              "name": "include",
              "locations": [
                "FIELD",
                "FRAGMENT_SPREAD",
                "INLINE_FRAGMENT"
              ],
              "args": [
                {
                  "name": "if",
                  "type": {
                    "kind": "NON_NULL",
                    "name": Null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": Null
                    }
                  },
                  "defaultValue": Null
                }
              ]
            }
          ]
        }
    });
    sort_schema_value(&mut v);
    v
}
