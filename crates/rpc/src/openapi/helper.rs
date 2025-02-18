use std::collections::HashSet;

use utoipa::{
    openapi::{
        Components, ContentBuilder, ObjectBuilder, RefOr, Response, ResponseBuilder, Schema,
        SchemaType,
    },
    OpenApi,
};

use super::docs::ApiDoc;

const JSON_CONTENT_TYPE: &str = "application/json";

pub(crate) fn add_string_property(
    builder: ObjectBuilder,
    name: &str,
    value: &str,
    description: &str,
) -> ObjectBuilder {
    let string_object = ObjectBuilder::new()
        .schema_type(SchemaType::String)
        .description(Some(description.to_string()))
        .enum_values(Some(vec![value.to_string()]))
        .build();

    let string_schema = RefOr::T(Schema::Object(string_object));
    builder.property(name, string_schema)
}

pub(crate) fn build_error_response(description: &str) -> Response {
    ResponseBuilder::new()
        .description(description)
        .content(
            JSON_CONTENT_TYPE,
            ContentBuilder::new()
                .schema(Schema::Object(
                    ObjectBuilder::new()
                        .property(
                            "error",
                            RefOr::T(Schema::Object(
                                ObjectBuilder::new().schema_type(SchemaType::String).build(),
                            )),
                        )
                        .build(),
                ))
                .build(),
        )
        .build()
}

pub(crate) fn request_schema(name: &str, params: Option<RefOr<Schema>>) -> RefOr<Schema> {
    let mut builder = ObjectBuilder::new();

    builder =
        add_string_property(builder, "jsonrpc", "2.0", "The version of the JSON-RPC protocol.");
    builder = add_string_property(builder, "id", "test-account", "An ID to identify the request.");
    builder = add_string_property(builder, "method", name, "The name of the method to invoke.");
    builder = builder.required("jsonrpc").required("id").required("method");

    if let Some(params) = params {
        builder = builder.property("params", params);
        builder = builder.required("params");
    }

    RefOr::T(Schema::Object(builder.build()))
}

pub(crate) fn find_all_components(schema: RefOr<Schema>) -> HashSet<String> {
    let mut components = HashSet::new();

    match schema {
        RefOr::T(schema) => match schema {
            Schema::Object(object) => {
                for (_, value) in object.properties {
                    components.extend(find_all_components(value));
                }
            }
            Schema::Array(array) => {
                components.extend(find_all_components(*array.items));
            }
            Schema::AllOf(all_of) => {
                for item in all_of.items {
                    components.extend(find_all_components(item));
                }
            }
            Schema::OneOf(one_of) => {
                for item in one_of.items {
                    components.extend(find_all_components(item));
                }
            }
            Schema::AnyOf(any_of) => {
                for item in any_of.items {
                    components.extend(find_all_components(item));
                }
            }
            _ => {}
        },
        RefOr::Ref(ref_location) => {
            components
                .insert(ref_location.ref_location.split('/').next_back().unwrap().to_string());
        }
    }

    components
}

pub(crate) fn filter_unused_components(
    request: Option<RefOr<Schema>>,
    response: RefOr<Schema>,
    components: &mut Components,
) {
    let mut used_components = request.map(find_all_components).unwrap_or_default();
    used_components.extend(find_all_components(response));

    let mut check_stack = used_components.clone();
    while !check_stack.is_empty() {
        let current = check_stack.iter().next().unwrap().clone();
        check_stack.remove(&current);

        if let Some(schema) = components.schemas.get(&current) {
            let child_components = find_all_components(schema.clone());
            for child in child_components {
                if !used_components.contains(&child) {
                    used_components.insert(child.clone());
                    check_stack.insert(child);
                }
            }
        }
    }

    components.schemas = components
        .schemas
        .iter()
        .filter(|(k, _)| used_components.contains(*k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
}

#[allow(non_snake_case)]
pub(crate) fn fix_examples_for_allOf_references(schema: RefOr<Schema>) -> RefOr<Schema> {
    match schema {
        RefOr::T(mut schema) => match schema {
            Schema::Object(ref mut object) => RefOr::T(Schema::Object({
                // Handle object properties recursively
                object.properties = object
                    .properties
                    .iter()
                    .map(|(key, value)| {
                        (key.clone(), fix_examples_for_allOf_references(value.clone()))
                    })
                    .collect();
                object.clone()
            })),
            Schema::Array(ref mut array) => RefOr::T(Schema::Array({
                // Handle array items recursively
                array.items = Box::new(fix_examples_for_allOf_references(*array.items.clone()));
                array.clone()
            })),
            Schema::AllOf(ref all_of) => {
                // If we have multiple items in allOf, we need to merge them
                if all_of.items.len() > 1 {
                    // Merge the schemas
                    let mut merged = all_of.items[0].clone();
                    for item in all_of.items.iter().skip(1) {
                        if let (
                            RefOr::T(Schema::Object(ref mut merged_obj)),
                            RefOr::T(Schema::Object(ref item_obj)),
                        ) = (merged.clone(), item.clone())
                        {
                            // Merge properties
                            merged_obj.properties.extend(item_obj.properties.clone());
                            // Merge required fields
                            merged_obj.required.extend(item_obj.required.clone());
                            merged = RefOr::T(Schema::Object(merged_obj.clone()));
                        }
                    }
                    merged
                } else {
                    // If only one item, just return it
                    all_of.items[0].clone()
                }
            }
            _ => RefOr::T(schema),
        },
        RefOr::Ref(_) => schema,
    }
}

pub(crate) fn add_referenced_components(schema: RefOr<Schema>, components: &mut Components) {
    match schema {
        RefOr::T(Schema::Object(obj)) => {
            // Process object properties
            for (_, prop) in obj.properties {
                add_referenced_components(prop, components);
            }
        }
        RefOr::T(Schema::Array(arr)) => {
            // Process array items
            add_referenced_components(*arr.items, components);
        }
        RefOr::T(Schema::AllOf(all_of)) => {
            // Process allOf schemas
            for item in all_of.items {
                add_referenced_components(item, components);
            }
        }
        RefOr::Ref(reference) => {
            // Extract component name from reference
            let component_name = reference.ref_location.split('/').next_back().unwrap().to_string();

            // If we haven't already added this component
            if !components.schemas.contains_key(&component_name) {
                // Get the component schema from ApiDoc
                if let Some(schema) =
                    ApiDoc::openapi().components.unwrap().schemas.get(&component_name)
                {
                    // Add it to our components
                    components.schemas.insert(component_name.clone(), schema.clone());

                    // Recursively process any nested references in this component
                    add_referenced_components(schema.clone(), components);
                }
            }
        }
        _ => {}
    }
}
