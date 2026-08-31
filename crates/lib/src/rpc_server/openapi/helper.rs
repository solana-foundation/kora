use utoipa::openapi::{
    schema::Type, ContentBuilder, ObjectBuilder, RefOr, Response, ResponseBuilder, Schema,
};

const JSON_CONTENT_TYPE: &str = "application/json";

pub(crate) fn add_string_property(
    builder: ObjectBuilder,
    name: &str,
    value: &str,
    description: &str,
) -> ObjectBuilder {
    let string_object = ObjectBuilder::new()
        .schema_type(Type::String)
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
                .schema(Some(Schema::Object(
                    ObjectBuilder::new()
                        .property(
                            "error",
                            RefOr::T(Schema::Object(
                                ObjectBuilder::new().schema_type(Type::String).build(),
                            )),
                        )
                        .build(),
                )))
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
