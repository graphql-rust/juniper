use crate::value::Value;

pub(crate) fn schema_introspection_result() -> Value {
    graphql_value!({
        "__schema": {
          "description": null,
          "queryType": {
            "name": "Query"
          },
          "mutationType": null,
          "subscriptionType": null,
          "types": [
            {
              "kind": "ENUM",
              "name": "Episode",
              "description": null,
              "specifiedByUrl": null,
              "fields": null,
              "inputFields": null,
              "interfaces": null,
              "enumValues": [
                {
                  "name": "NEW_HOPE",
                  "description": null,
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "EMPIRE",
                  "description": null,
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "JEDI",
                  "description": null,
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "possibleTypes": null
            },
            {
              "kind": "ENUM",
              "name": "__DirectiveLocation",
              "description": null,
              "specifiedByUrl": null,
              "fields": null,
              "inputFields": null,
              "interfaces": null,
              "enumValues": [
                {
                  "name": "QUERY",
                  "description": null,
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "MUTATION",
                  "description": null,
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "SUBSCRIPTION",
                  "description": null,
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "FIELD",
                  "description": null,
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "SCALAR",
                  "description": null,
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "FRAGMENT_DEFINITION",
                  "description": null,
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "FIELD_DEFINITION",
                  "description": null,
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "VARIABLE_DEFINITION",
                  "description": null,
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "FRAGMENT_SPREAD",
                  "description": null,
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "INLINE_FRAGMENT",
                  "description": null,
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "ENUM_VALUE",
                  "description": null,
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "possibleTypes": null
            },
            {
              "kind": "ENUM",
              "name": "__TypeKind",
              "description": "GraphQL type kind\n\nThe GraphQL specification defines a number of type kinds - the meta type of a type.",
              "specifiedByUrl": null,
              "fields": null,
              "inputFields": null,
              "interfaces": null,
              "enumValues": [
                {
                  "name": "SCALAR",
                  "description": "## Scalar types\n\nScalar types appear as the leaf nodes of GraphQL queries. Strings, numbers, and booleans are the built in types, and while it's possible to define your own, it's relatively uncommon.",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "OBJECT",
                  "description": "## Object types\n\nThe most common type to be implemented by users. Objects have fields and can implement interfaces.",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "INTERFACE",
                  "description": "## Interface types\n\nInterface types are used to represent overlapping fields between multiple types, and can be queried for their concrete type.",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "UNION",
                  "description": "## Union types\n\nUnions are similar to interfaces but can not contain any fields on their own.",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "ENUM",
                  "description": "## Enum types\n\nLike scalars, enum types appear as the leaf nodes of GraphQL queries.",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "INPUT_OBJECT",
                  "description": "## Input objects\n\nRepresents complex values provided in queries _into_ the system.",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "LIST",
                  "description": "## List types\n\nRepresent lists of other types. This library provides implementations for vectors and slices, but other Rust types can be extended to serve as GraphQL lists.",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "NON_NULL",
                  "description": "## Non-null types\n\nIn GraphQL, nullable types are the default. By putting a `!` after a type, it becomes non-nullable.",
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "possibleTypes": null
            },
            {
              "kind": "INTERFACE",
              "name": "Character",
              "description": "A character in the Star Wars Trilogy",
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "id",
                  "description": "The id of the character",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "name",
                  "description": "The name of the character",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "friends",
                  "description": "The friends of the character",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "INTERFACE",
                          "name": "Character",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "appearsIn",
                  "description": "Which movies they appear in",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "Episode",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "inputFields": null,
              "interfaces": [],
              "enumValues": null,
              "possibleTypes": [
                {
                  "kind": "OBJECT",
                  "name": "Droid",
                  "ofType": null
                },
                {
                  "kind": "OBJECT",
                  "name": "Human",
                  "ofType": null
                },
              ]
            },
            {
              "kind": "SCALAR",
              "name": "Boolean",
              "description": null,
              "specifiedByUrl": null,
              "fields": null,
              "inputFields": null,
              "interfaces": null,
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "SCALAR",
              "name": "String",
              "description": null,
              "specifiedByUrl": null,
              "fields": null,
              "inputFields": null,
              "interfaces": null,
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "OBJECT",
              "name": "Droid",
              "description": "A mechanical creature in the Star Wars universe.",
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "id",
                  "description": "The id of the droid",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "name",
                  "description": "The name of the droid",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "friends",
                  "description": "The friends of the droid",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "INTERFACE",
                          "name": "Character",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "appearsIn",
                  "description": "Which movies they appear in",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "Episode",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "primaryFunction",
                  "description": "The primary function of the droid",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "inputFields": null,
              "interfaces": [
                {
                  "kind": "INTERFACE",
                  "name": "Character",
                  "ofType": null
                }
              ],
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "OBJECT",
              "name": "Human",
              "description": "A humanoid creature in the Star Wars universe.",
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "id",
                  "description": "The id of the human",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "name",
                  "description": "The name of the human",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "friends",
                  "description": "The friends of the human",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "INTERFACE",
                          "name": "Character",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "appearsIn",
                  "description": "Which movies they appear in",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "Episode",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "homePlanet",
                  "description": "The home planet of the human",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "inputFields": null,
              "interfaces": [
                {
                  "kind": "INTERFACE",
                  "name": "Character",
                  "ofType": null
                }
              ],
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "OBJECT",
              "name": "Query",
              "description": "The root query object of the schema",
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "human",
                  "description": null,
                  "args": [
                    {
                      "name": "id",
                      "description": "id of the human",
                      "type": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "SCALAR",
                          "name": "String",
                          "ofType": null
                        }
                      },
                      "defaultValue": null
                    }
                  ],
                  "type": {
                    "kind": "OBJECT",
                    "name": "Human",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "droid",
                  "description": null,
                  "args": [
                    {
                      "name": "id",
                      "description": null,
                      "description": "id of the droid",
                      "type": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "SCALAR",
                          "name": "String",
                          "ofType": null
                        }
                      },
                      "defaultValue": null
                    }
                  ],
                  "type": {
                    "kind": "OBJECT",
                    "name": "Droid",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "hero",
                  "description": null,
                  "args": [
                    {
                      "name": "episode",
                      "description": "If omitted, returns the hero of the whole saga. If provided, returns the hero of that particular episode",
                      "type": {
                        "kind": "ENUM",
                        "name": "Episode",
                        "ofType": null
                      },
                      "defaultValue": null
                    }
                  ],
                  "type": {
                    "kind": "INTERFACE",
                    "name": "Character",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "inputFields": null,
              "interfaces": [],
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "OBJECT",
              "name": "__Directive",
              "description": null,
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "name",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "description",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "locations",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "__DirectiveLocation",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "isRepeatable",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "args",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__InputValue",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "onOperation",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": null
                    }
                  },
                  "isDeprecated": true,
                  "deprecationReason": "Use the locations array instead"
                },
                {
                  "name": "onFragment",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": null
                    }
                  },
                  "isDeprecated": true,
                  "deprecationReason": "Use the locations array instead"
                },
                {
                  "name": "onField",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": null
                    }
                  },
                  "isDeprecated": true,
                  "deprecationReason": "Use the locations array instead"
                }
              ],
              "inputFields": null,
              "interfaces": [],
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "OBJECT",
              "name": "__EnumValue",
              "description": null,
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "name",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "description",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "isDeprecated",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "deprecationReason",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "inputFields": null,
              "interfaces": [],
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "OBJECT",
              "name": "__Field",
              "description": null,
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "name",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "description",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "args",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__InputValue",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "type",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "OBJECT",
                      "name": "__Type",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "isDeprecated",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "deprecationReason",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "inputFields": null,
              "interfaces": [],
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "OBJECT",
              "name": "__InputValue",
              "description": null,
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "name",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "description",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "type",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "OBJECT",
                      "name": "__Type",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "defaultValue",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "inputFields": null,
              "interfaces": [],
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "OBJECT",
              "name": "__Schema",
              "description": null,
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "description",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "types",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__Type",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "queryType",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "OBJECT",
                      "name": "__Type",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "mutationType",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "OBJECT",
                    "name": "__Type",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "subscriptionType",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "OBJECT",
                    "name": "__Type",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "directives",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__Directive",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "inputFields": null,
              "interfaces": [],
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "OBJECT",
              "name": "__Type",
              "description": null,
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "name",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "description",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "specifiedByUrl",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "kind",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "ENUM",
                      "name": "__TypeKind",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "fields",
                  "description": null,
                  "args": [
                    {
                      "name": "includeDeprecated",
                      "description": null,
                      "type": {
                        "kind": "SCALAR",
                        "name": "Boolean",
                        "ofType": null
                      },
                      "defaultValue": "false"
                    }
                  ],
                  "type": {
                    "kind": "LIST",
                    "name": null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__Field",
                        "ofType": null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "ofType",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "OBJECT",
                    "name": "__Type",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "inputFields",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "LIST",
                    "name": null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__InputValue",
                        "ofType": null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "interfaces",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "LIST",
                    "name": null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__Type",
                        "ofType": null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "possibleTypes",
                  "description": null,
                  "args": [],
                  "type": {
                    "kind": "LIST",
                    "name": null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__Type",
                        "ofType": null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "enumValues",
                  "description": null,
                  "args": [
                    {
                      "name": "includeDeprecated",
                      "description": null,
                      "type": {
                        "kind": "SCALAR",
                        "name": "Boolean",
                        "ofType": null
                      },
                      "defaultValue": "false"
                    }
                  ],
                  "type": {
                    "kind": "LIST",
                    "name": null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__EnumValue",
                        "ofType": null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "inputFields": null,
              "interfaces": [],
              "enumValues": null,
              "possibleTypes": null
            },
          ],
          "directives": [
            {
              "name": "deprecated",
              "description": null,
              "isRepeatable": false,
              "locations": [
                "FIELD_DEFINITION",
                "ENUM_VALUE"
              ],
              "args": [
                {
                  "name": "reason",
                  "description": null,
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": null
                    }
                  },
                  "defaultValue": null
                }
              ]
            },
            {
              "name": "include",
              "description": null,
              "isRepeatable": false,
              "locations": [
                "FIELD",
                "FRAGMENT_SPREAD",
                "INLINE_FRAGMENT"
              ],
              "args": [
                {
                  "name": "if",
                  "description": null,
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": null
                    }
                  },
                  "defaultValue": null
                }
              ]
            },
            {
              "name": "skip",
              "description": null,
              "isRepeatable": false,
              "locations": [
                "FIELD",
                "FRAGMENT_SPREAD",
                "INLINE_FRAGMENT"
              ],
              "args": [
                {
                  "name": "if",
                  "description": null,
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": null
                    }
                  },
                  "defaultValue": null
                }
              ]
            },
            {
              "name": "specifiedBy",
              "description": null,
              "isRepeatable": false,
              "locations": [
                "SCALAR"
              ],
              "args": [
                {
                  "name": "url",
                  "description": null,
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": null
                    }
                  },
                  "defaultValue": null
                }
              ]
            }
          ]
        }
    })
}

pub(crate) fn schema_introspection_result_without_descriptions() -> Value {
    graphql_value!({
        "__schema": {
          "queryType": {
            "name": "Query"
          },
          "mutationType": null,
          "subscriptionType": null,
          "types": [
            {
              "kind": "ENUM",
              "name": "Episode",
              "specifiedByUrl": null,
              "fields": null,
              "inputFields": null,
              "interfaces": null,
              "enumValues": [
                {
                  "name": "NEW_HOPE",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "EMPIRE",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "JEDI",
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "possibleTypes": null
            },
            {
              "kind": "ENUM",
              "name": "__DirectiveLocation",
              "specifiedByUrl": null,
              "fields": null,
              "inputFields": null,
              "interfaces": null,
              "enumValues": [
                {
                  "name": "QUERY",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "MUTATION",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "SUBSCRIPTION",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "FIELD",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "SCALAR",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "FRAGMENT_DEFINITION",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "FIELD_DEFINITION",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "VARIABLE_DEFINITION",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "FRAGMENT_SPREAD",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "INLINE_FRAGMENT",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "ENUM_VALUE",
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "possibleTypes": null
            },
            {
              "kind": "ENUM",
              "name": "__TypeKind",
              "specifiedByUrl": null,
              "fields": null,
              "inputFields": null,
              "interfaces": null,
              "enumValues": [
                {
                  "name": "SCALAR",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "OBJECT",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "INTERFACE",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "UNION",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "ENUM",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "INPUT_OBJECT",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "LIST",
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "NON_NULL",
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "possibleTypes": null
            },
            {
              "kind": "INTERFACE",
              "name": "Character",
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "id",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "name",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "friends",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "INTERFACE",
                          "name": "Character",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "appearsIn",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "Episode",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "inputFields": null,
              "interfaces": [],
              "enumValues": null,
              "possibleTypes": [
                {
                  "kind": "OBJECT",
                  "name": "Droid",
                  "ofType": null
                },
                {
                  "kind": "OBJECT",
                  "name": "Human",
                  "ofType": null
                },
              ]
            },
            {
              "kind": "SCALAR",
              "name": "Boolean",
              "specifiedByUrl": null,
              "fields": null,
              "inputFields": null,
              "interfaces": null,
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "SCALAR",
              "name": "String",
              "specifiedByUrl": null,
              "fields": null,
              "inputFields": null,
              "interfaces": null,
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "OBJECT",
              "name": "Droid",
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "id",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "name",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "friends",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "INTERFACE",
                          "name": "Character",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "appearsIn",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "Episode",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "primaryFunction",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "inputFields": null,
              "interfaces": [
                {
                  "kind": "INTERFACE",
                  "name": "Character",
                  "ofType": null
                }
              ],
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "OBJECT",
              "name": "Human",
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "id",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "name",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "friends",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "INTERFACE",
                          "name": "Character",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "appearsIn",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "Episode",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "homePlanet",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "inputFields": null,
              "interfaces": [
                {
                  "kind": "INTERFACE",
                  "name": "Character",
                  "ofType": null
                }
              ],
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "OBJECT",
              "name": "Query",
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "human",
                  "args": [
                    {
                      "name": "id",
                      "type": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "SCALAR",
                          "name": "String",
                          "ofType": null
                        }
                      },
                      "defaultValue": null
                    }
                  ],
                  "type": {
                    "kind": "OBJECT",
                    "name": "Human",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "droid",
                  "args": [
                    {
                      "name": "id",
                      "type": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "SCALAR",
                          "name": "String",
                          "ofType": null
                        }
                      },
                      "defaultValue": null
                    }
                  ],
                  "type": {
                    "kind": "OBJECT",
                    "name": "Droid",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "hero",
                  "args": [
                    {
                      "name": "episode",
                      "type": {
                        "kind": "ENUM",
                        "name": "Episode",
                        "ofType": null
                      },
                      "defaultValue": null
                    }
                  ],
                  "type": {
                    "kind": "INTERFACE",
                    "name": "Character",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "inputFields": null,
              "interfaces": [],
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "OBJECT",
              "name": "__Directive",
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "name",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "description",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "locations",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "ENUM",
                          "name": "__DirectiveLocation",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "isRepeatable",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "args",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__InputValue",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "onOperation",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": null
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
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": null
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
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": null
                    }
                  },
                  "isDeprecated": true,
                  "deprecationReason": "Use the locations array instead"
                },
              ],
              "inputFields": null,
              "interfaces": [],
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "OBJECT",
              "name": "__EnumValue",
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "name",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "description",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "isDeprecated",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "deprecationReason",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "inputFields": null,
              "interfaces": [],
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "OBJECT",
              "name": "__Field",
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "name",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "description",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "args",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__InputValue",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "type",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "OBJECT",
                      "name": "__Type",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "isDeprecated",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "deprecationReason",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "inputFields": null,
              "interfaces": [],
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "OBJECT",
              "name": "__InputValue",
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "name",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "description",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "type",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "OBJECT",
                      "name": "__Type",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "defaultValue",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "inputFields": null,
              "interfaces": [],
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "OBJECT",
              "name": "__Schema",
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "description",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "types",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__Type",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "queryType",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "OBJECT",
                      "name": "__Type",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "mutationType",
                  "args": [],
                  "type": {
                    "kind": "OBJECT",
                    "name": "__Type",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "subscriptionType",
                  "args": [],
                  "type": {
                    "kind": "OBJECT",
                    "name": "__Type",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "directives",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "LIST",
                      "name": null,
                      "ofType": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {
                          "kind": "OBJECT",
                          "name": "__Directive",
                          "ofType": null
                        }
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "inputFields": null,
              "interfaces": [],
              "enumValues": null,
              "possibleTypes": null
            },
            {
              "kind": "OBJECT",
              "name": "__Type",
              "specifiedByUrl": null,
              "fields": [
                {
                  "name": "name",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "description",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "specifiedByUrl",
                  "args": [],
                  "type": {
                    "kind": "SCALAR",
                    "name": "String",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "kind",
                  "args": [],
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "ENUM",
                      "name": "__TypeKind",
                      "ofType": null
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "fields",
                  "args": [
                    {
                      "name": "includeDeprecated",
                      "type": {
                        "kind": "SCALAR",
                        "name": "Boolean",
                        "ofType": null
                      },
                      "defaultValue": "false"
                    }
                  ],
                  "type": {
                    "kind": "LIST",
                    "name": null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__Field",
                        "ofType": null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "ofType",
                  "args": [],
                  "type": {
                    "kind": "OBJECT",
                    "name": "__Type",
                    "ofType": null
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "inputFields",
                  "args": [],
                  "type": {
                    "kind": "LIST",
                    "name": null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__InputValue",
                        "ofType": null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "interfaces",
                  "args": [],
                  "type": {
                    "kind": "LIST",
                    "name": null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__Type",
                        "ofType": null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "possibleTypes",
                  "args": [],
                  "type": {
                    "kind": "LIST",
                    "name": null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__Type",
                        "ofType": null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                },
                {
                  "name": "enumValues",
                  "args": [
                    {
                      "name": "includeDeprecated",
                      "type": {
                        "kind": "SCALAR",
                        "name": "Boolean",
                        "ofType": null
                      },
                      "defaultValue": "false"
                    }
                  ],
                  "type": {
                    "kind": "LIST",
                    "name": null,
                    "ofType": {
                      "kind": "NON_NULL",
                      "name": null,
                      "ofType": {
                        "kind": "OBJECT",
                        "name": "__EnumValue",
                        "ofType": null
                      }
                    }
                  },
                  "isDeprecated": false,
                  "deprecationReason": null
                }
              ],
              "inputFields": null,
              "interfaces": [],
              "enumValues": null,
              "possibleTypes": null
            },
          ],
          "directives": [
            {
              "name": "deprecated",
              "isRepeatable": false,
              "locations": [
                "FIELD_DEFINITION",
                "ENUM_VALUE"
              ],
              "args": [
                {
                  "name": "reason",
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": null
                    }
                  },
                  "defaultValue": null
                }
              ]
            },
            {
              "name": "include",
              "isRepeatable": false,
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
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": null
                    }
                  },
                  "defaultValue": null
                }
              ]
            },
            {
              "name": "skip",
              "isRepeatable": false,
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
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "Boolean",
                      "ofType": null
                    }
                  },
                  "defaultValue": null
                }
              ]
            },
            {
              "name": "specifiedBy",
              "isRepeatable": false,
              "locations": [
                "SCALAR"
              ],
              "args": [
                {
                  "name": "url",
                  "type": {
                    "kind": "NON_NULL",
                    "name": null,
                    "ofType": {
                      "kind": "SCALAR",
                      "name": "String",
                      "ofType": null
                    }
                  },
                  "defaultValue": null
                }
              ]
            }
          ]
        }
    })
}
