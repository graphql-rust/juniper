use crate::value::Value;

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

pub(crate) fn schema_introspection_result() -> Value {
    let mut v = graphql_value!({
        "__schema": {
          "queryType": {
            "name": "Query"
          },
          "mutationType": None,
          "subscriptionType": None,
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
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "name",
                  "description": "The name of the human",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "friends",
                  "description": "The friends of the human",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "INTERFACE",
                          "name": "Character",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "appearsIn",
                  "description": "Which movies they appear in",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "Episode",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "homePlanet",
                  "description": "The home planet of the human",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "inputFields": None,
              "interfaces": [
                {
                  "kind": "INTERFACE",
                  "name": "Character",
                  "ofType": None
                }
              ],
              "enumValues": None,
              "possibleTypes": None
            },
            {
              "kind": "SCALAR",
              "name": "Boolean",
              "description": None,
              "fields": None,
              "inputFields": None,
              "interfaces": None,
              "enumValues": None,
              "possibleTypes": None
            },
            {
              "kind": "OBJECT",
              "name": "__InputValue",
              "description": None,
              "fields": [
                {
                  "name": "name",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "description",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "type",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "OBJECT",
                      "name": "__Type",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "defaultValue",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "inputFields": None,
              "interfaces": [],
              "enumValues": None,
              "possibleTypes": None
            },
            {
              "kind": "SCALAR",
              "name": "String",
              "description": None,
              "fields": None,
              "inputFields": None,
              "interfaces": None,
              "enumValues": None,
              "possibleTypes": None
            },
            {
              "kind": "OBJECT",
              "name": "__Field",
              "description": None,
              "fields": [
                {
                  "name": "name",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "description",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "args",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__InputValue",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "type",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "OBJECT",
                      "name": "__Type",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "isDeprecated",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "deprecationReason",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "inputFields": None,
              "interfaces": [],
              "enumValues": None,
              "possibleTypes": None
            },
            {
              "kind": "ENUM",
              "name": "__TypeKind",
              "description": "GraphQL type kind\n\nThe GraphQL specification defines a number of type kinds - the meta type of a type.",
              "fields": None,
              "inputFields": None,
              "interfaces": None,
              "enumValues": [
                {
                  "name": "SCALAR",
                  "description": "## Scalar types\n\nScalar types appear as the leaf nodes of GraphQL queries. Strings, numbers, and booleans are the built in types, and while it's possible to define your own, it's relatively uncommon.",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "OBJECT",
                  "description": "## Object types\n\nThe most common type to be implemented by users. Objects have fields and can implement interfaces.",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "INTERFACE",
                  "description": "## Interface types\n\nInterface types are used to represent overlapping fields between multiple types, and can be queried for their concrete type.",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "UNION",
                  "description": "## Union types\n\nUnions are similar to interfaces but can not contain any fields on their own.",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "ENUM",
                  "description": "## Enum types\n\nLike scalars, enum types appear as the leaf nodes of GraphQL queries.",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "INPUT_OBJECT",
                  "description": "## Input objects\n\nRepresents complex values provided in queries _into_ the system.",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "LIST",
                  "description": "## List types\n\nRepresent lists of other types. This library provides implementations for vectors and slices, but other Rust types can be extended to serve as GraphQL lists.",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "NON_NULL",
                  "description": "## Non-null types\n\nIn GraphQL, nullable types are the default. By putting a `!` after a type, it becomes non-nullable.",
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "possibleTypes": None
            },
            {
              "kind": "OBJECT",
              "name": "__Type",
              "description": None,
              "fields": [
                {
                  "name": "name",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "description",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "kind",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "ENUM",
                      "name": "__TypeKind",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "fields",
                  "description": None,
                  "args": [
                    {
                      "name": "includeDeprecated",
                      "description": None,
                      "type": {
                        "kind": "SCALAR",
                        "name": "Boolean",
                        "ofType": None
                      },
                      "defaultValue": "false"
                    }
                  ],
                  "type": {
                    "kind": "LIST",
                    "name": None,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": None,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__Field",
                        "ofType": None
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "ofType",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "OBJECT",
                    "name": "__Type",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "inputFields",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "LIST",
                    "name": None,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": None,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__InputValue",
                        "ofType": None
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "interfaces",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "LIST",
                    "name": None,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": None,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__Type",
                        "ofType": None
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "possibleTypes",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "LIST",
                    "name": None,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": None,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__Type",
                        "ofType": None
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "enumValues",
                  "description": None,
                  "args": [
                    {
                      "name": "includeDeprecated",
                      "description": None,
                      "type": {
                        "kind": "SCALAR",
                        "name": "Boolean",
                        "ofType": None
                      },
                      "defaultValue": "false"
                    }
                  ],
                  "type": {
                    "kind": "LIST",
                    "name": None,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": None,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__EnumValue",
                        "ofType": None
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "inputFields": None,
              "interfaces": [],
              "enumValues": None,
              "possibleTypes": None
            },
            {
              "kind": "OBJECT",
              "name": "__Schema",
              "description": None,
              "fields": [
                {
                  "name": "types",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__Type",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "queryType",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "OBJECT",
                      "name": "__Type",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "mutationType",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "OBJECT",
                    "name": "__Type",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "subscriptionType",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "OBJECT",
                    "name": "__Type",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "directives",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__Directive",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "inputFields": None,
              "interfaces": [],
              "enumValues": None,
              "possibleTypes": None
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
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "name",
                  "description": "The name of the droid",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "friends",
                  "description": "The friends of the droid",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "INTERFACE",
                          "name": "Character",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "appearsIn",
                  "description": "Which movies they appear in",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "Episode",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "primaryFunction",
                  "description": "The primary function of the droid",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "inputFields": None,
              "interfaces": [
                {
                  "kind": "INTERFACE",
                  "name": "Character",
                  "ofType": None
                }
              ],
              "enumValues": None,
              "possibleTypes": None
            },
            {
              "kind": "OBJECT",
              "name": "Query",
              "description": "The root query object of the schema",
              "fields": [
                {
                  "name": "human",
                  "description": None,
                  "args": [
                    {
                      "name": "id",
                      "description": "id of the human",
                      "type": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "SCALAR",
                          "name": "String",
                          "ofType": None
                        }
                      },
                      "defaultValue": None
                    }
                  ],
                  "type": {
                    "kind": "OBJECT",
                    "name": "Human",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "droid",
                  "description": None,
                  "args": [
                    {
                      "name": "id",
                      "description": None,
                      "description": "id of the droid",
                      "type": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "SCALAR",
                          "name": "String",
                          "ofType": None
                        }
                      },
                      "defaultValue": None
                    }
                  ],
                  "type": {
                    "kind": "OBJECT",
                    "name": "Droid",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "hero",
                  "description": None,
                  "args": [
                    {
                      "name": "episode",
                      "description": "If omitted, returns the hero of the whole saga. If provided, returns the hero of that particular episode",
                      "type": {
                        "kind": "ENUM",
                        "name": "Episode",
                        "ofType": None
                      },
                      "defaultValue": None
                    }
                  ],
                  "type": {
                    "kind": "INTERFACE",
                    "name": "Character",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "inputFields": None,
              "interfaces": [],
              "enumValues": None,
              "possibleTypes": None
            },
            {
              "kind": "OBJECT",
              "name": "__EnumValue",
              "description": None,
              "fields": [
                {
                  "name": "name",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "description",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "isDeprecated",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "deprecationReason",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "inputFields": None,
              "interfaces": [],
              "enumValues": None,
              "possibleTypes": None
            },
            {
              "kind": "ENUM",
              "name": "Episode",
              "description": None,
              "fields": None,
              "inputFields": None,
              "interfaces": None,
              "enumValues": [
                {
                  "name": "NEW_HOPE",
                  "description": None,
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "EMPIRE",
                  "description": None,
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "JEDI",
                  "description": None,
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "possibleTypes": None
            },
            {
              "kind": "ENUM",
              "name": "__DirectiveLocation",
              "description": None,
              "fields": None,
              "inputFields": None,
              "interfaces": None,
              "enumValues": [
                {
                  "name": "QUERY",
                  "description": None,
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "MUTATION",
                  "description": None,
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "SUBSCRIPTION",
                  "description": None,
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "FIELD",
                  "description": None,
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "FRAGMENT_DEFINITION",
                  "description": None,
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "FRAGMENT_SPREAD",
                  "description": None,
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "INLINE_FRAGMENT",
                  "description": None,
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "possibleTypes": None
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
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "name",
                  "description": "The name of the character",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "friends",
                  "description": "The friends of the character",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "INTERFACE",
                          "name": "Character",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "appearsIn",
                  "description": "Which movies they appear in",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "Episode",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "inputFields": None,
              "interfaces": None,
              "enumValues": None,
              "possibleTypes": [
                {
                  "kind": "OBJECT",
                  "name": "Human",
                  "ofType": None
                },
                {
                  "kind": "OBJECT",
                  "name": "Droid",
                  "ofType": None
                }
              ]
            },
            {
              "kind": "OBJECT",
              "name": "__Directive",
              "description": None,
              "fields": [
                {
                  "name": "name",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "description",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "locations",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "__DirectiveLocation",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "args",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__InputValue",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "onOperation",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": None
                    }
                  },
                  "isDeprecated": true,
                  "deprecationReason": "Use the locations array instead"
                },
                {
                  "name": "onFragment",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": None
                    }
                  },
                  "isDeprecated": true,
                  "deprecationReason": "Use the locations array instead"
                },
                {
                  "name": "onField",
                  "description": None,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": None
                    }
                  },
                  "isDeprecated": true,
                  "deprecationReason": "Use the locations array instead"
                }
              ],
              "inputFields": None,
              "interfaces": [],
              "enumValues": None,
              "possibleTypes": None
            }
          ],
          "directives": [
            {
              "name": "skip",
              "description": None,
              "locations": [
                "FIELD",
                "FRAGMENT_SPREAD",
                "INLINE_FRAGMENT"
              ],
              "args": [
                {
                  "name": "if",
                  "description": None,
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": None
                    }
                  },
                  "defaultValue": None
                }
              ]
            },
            {
              "name": "include",
              "description": None,
              "locations": [
                "FIELD",
                "FRAGMENT_SPREAD",
                "INLINE_FRAGMENT"
              ],
              "args": [
                {
                  "name": "if",
                  "description": None,
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": None
                    }
                  },
                  "defaultValue": None
                }
              ]
            }
          ]
        }
    });
    sort_schema_value(&mut v);
    v
}

pub(crate) fn schema_introspection_result_without_descriptions() -> Value {
    let mut v = graphql_value!({
        "__schema": {
          "queryType": {
            "name": "Query"
          },
          "mutationType": None,
          "subscriptionType": None,
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
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "name",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "friends",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "INTERFACE",
                          "name": "Character",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "appearsIn",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "Episode",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "homePlanet",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "inputFields": None,
              "interfaces": [
                {
                  "kind": "INTERFACE",
                  "name": "Character",
                  "ofType": None
                }
              ],
              "enumValues": None,
              "possibleTypes": None
            },
            {
              "kind": "SCALAR",
              "name": "Boolean",
              "fields": None,
              "inputFields": None,
              "interfaces": None,
              "enumValues": None,
              "possibleTypes": None
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
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "description",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "type",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "OBJECT",
                      "name": "__Type",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "defaultValue",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "inputFields": None,
              "interfaces": [],
              "enumValues": None,
              "possibleTypes": None
            },
            {
              "kind": "SCALAR",
              "name": "String",
              "fields": None,
              "inputFields": None,
              "interfaces": None,
              "enumValues": None,
              "possibleTypes": None
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
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "description",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "args",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__InputValue",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "type",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "OBJECT",
                      "name": "__Type",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "isDeprecated",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "deprecationReason",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "inputFields": None,
              "interfaces": [],
              "enumValues": None,
              "possibleTypes": None
            },
            {
              "kind": "ENUM",
              "name": "__TypeKind",
              "fields": None,
              "inputFields": None,
              "interfaces": None,
              "enumValues": [
                {
                  "name": "SCALAR",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "OBJECT",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "INTERFACE",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "UNION",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "ENUM",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "INPUT_OBJECT",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "LIST",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "NON_NULL",
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "possibleTypes": None
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
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "description",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "kind",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "ENUM",
                      "name": "__TypeKind",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "fields",
                  "args": [
                    {
                      "name": "includeDeprecated",
                      "type": {
                        "kind": "SCALAR",
                        "name": "Boolean",
                        "ofType": None
                      },
                      "defaultValue": "false"
                    }
                  ],
                  "type": {
                    "kind": "LIST",
                    "name": None,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": None,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__Field",
                        "ofType": None
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "ofType",
                  "args": [],
                  "type": {
                    "kind": "OBJECT",
                    "name": "__Type",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "inputFields",
                  "args": [],
                  "type": {
                    "kind": "LIST",
                    "name": None,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": None,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__InputValue",
                        "ofType": None
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "interfaces",
                  "args": [],
                  "type": {
                    "kind": "LIST",
                    "name": None,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": None,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__Type",
                        "ofType": None
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "possibleTypes",
                  "args": [],
                  "type": {
                    "kind": "LIST",
                    "name": None,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": None,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__Type",
                        "ofType": None
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "enumValues",
                  "args": [
                    {
                      "name": "includeDeprecated",
                      "type": {
                        "kind": "SCALAR",
                        "name": "Boolean",
                        "ofType": None
                      },
                      "defaultValue": "false"
                    }
                  ],
                  "type": {
                    "kind": "LIST",
                    "name": None,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": None,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__EnumValue",
                        "ofType": None
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "inputFields": None,
              "interfaces": [],
              "enumValues": None,
              "possibleTypes": None
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
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__Type",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "queryType",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "OBJECT",
                      "name": "__Type",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "mutationType",
                  "args": [],
                  "type": {
                    "kind": "OBJECT",
                    "name": "__Type",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "subscriptionType",
                  "args": [],
                  "type": {
                    "kind": "OBJECT",
                    "name": "__Type",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "directives",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__Directive",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "inputFields": None,
              "interfaces": [],
              "enumValues": None,
              "possibleTypes": None
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
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "name",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "friends",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "INTERFACE",
                          "name": "Character",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "appearsIn",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "Episode",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "primaryFunction",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "inputFields": None,
              "interfaces": [
                {
                  "kind": "INTERFACE",
                  "name": "Character",
                  "ofType": None
                }
              ],
              "enumValues": None,
              "possibleTypes": None
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
                        "name": None,
                        "ofType": {
                          "kind": "SCALAR",
                          "name": "String",
                          "ofType": None
                        }
                      },
                      "defaultValue": None
                    }
                  ],
                  "type": {
                    "kind": "OBJECT",
                    "name": "Human",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "droid",
                  "args": [
                    {
                      "name": "id",
                      "type": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "SCALAR",
                          "name": "String",
                          "ofType": None
                        }
                      },
                      "defaultValue": None
                    }
                  ],
                  "type": {
                    "kind": "OBJECT",
                    "name": "Droid",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "hero",
                  "args": [
                    {
                      "name": "episode",
                      "type": {
                        "kind": "ENUM",
                        "name": "Episode",
                        "ofType": None
                      },
                      "defaultValue": None
                    }
                  ],
                  "type": {
                    "kind": "INTERFACE",
                    "name": "Character",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "inputFields": None,
              "interfaces": [],
              "enumValues": None,
              "possibleTypes": None
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
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "description",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "isDeprecated",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "deprecationReason",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "inputFields": None,
              "interfaces": [],
              "enumValues": None,
              "possibleTypes": None
            },
            {
              "kind": "ENUM",
              "name": "Episode",
              "fields": None,
              "inputFields": None,
              "interfaces": None,
              "enumValues": [
                {
                  "name": "NEW_HOPE",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "EMPIRE",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "JEDI",
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "possibleTypes": None
            },
            {
              "kind": "ENUM",
              "name": "__DirectiveLocation",
              "fields": None,
              "inputFields": None,
              "interfaces": None,
              "enumValues": [
                {
                  "name": "QUERY",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "MUTATION",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "SUBSCRIPTION",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "FIELD",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "FRAGMENT_DEFINITION",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "FRAGMENT_SPREAD",
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "INLINE_FRAGMENT",
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "possibleTypes": None
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
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "name",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "friends",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "INTERFACE",
                          "name": "Character",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "appearsIn",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "Episode",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                }
              ],
              "inputFields": None,
              "interfaces": None,
              "enumValues": None,
              "possibleTypes": [
                {
                  "kind": "OBJECT",
                  "name": "Human",
                  "ofType": None
                },
                {
                  "kind": "OBJECT",
                  "name": "Droid",
                  "ofType": None
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
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": None
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "description",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": None
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "locations",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "__DirectiveLocation",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "args",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "LIST",
                      "name": None,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": None,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__InputValue",
                          "ofType": None
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": None
                },
                {
                  "name": "onOperation",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": None
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
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": None
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
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": None
                    }
                  },
                  "isDeprecated": true,
                  "deprecationReason": "Use the locations array instead"
                }
              ],
              "inputFields": None,
              "interfaces": [],
              "enumValues": None,
              "possibleTypes": None
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
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": None
                    }
                  },
                  "defaultValue": None
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
                    "name": None,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": None
                    }
                  },
                  "defaultValue": None
                }
              ]
            }
          ]
        }
    });
    sort_schema_value(&mut v);
    v
}
