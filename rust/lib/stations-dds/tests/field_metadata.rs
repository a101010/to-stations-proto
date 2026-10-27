use stations_dds::cyclonedds::{
    DdsFieldMetadata, DynamicPrimitiveKind, DynamicTypeSchema, DynamicValue,
};
use stations_dds::hello_world::{HelloWorldTopic, HelloWorldViewTopic};
use stations_dds::math::Quaternion;

fn quaternion() -> Quaternion {
    Quaternion {
        x: 0.25,
        y: 0.5,
        z: 0.75,
        w: 1.0,
    }
}

fn sample() -> HelloWorldTopic {
    HelloWorldTopic {
        greeting: "Hello, world!".to_string(),
        attitude: quaternion(),
    }
}

fn struct_fields(schema: &DynamicTypeSchema) -> &[stations_dds::cyclonedds::DynamicFieldSchema] {
    match schema {
        DynamicTypeSchema::Struct { fields, .. } => fields,
        other => panic!("expected struct schema, got {other:?}"),
    }
}

#[test]
fn enumerates_hello_world_topic_fields() {
    let schema = HelloWorldTopic::field_schema();
    let fields = struct_fields(schema);

    assert_eq!(
        HelloWorldTopic::field_names(),
        vec!["greeting".to_string(), "attitude".to_string()]
    );
    assert_eq!(
        HelloWorldTopic::field_names(),
        fields
            .iter()
            .map(|field| field.name.clone())
            .collect::<Vec<_>>()
    );

    match schema {
        DynamicTypeSchema::Struct { name, .. } => assert_eq!(name, "hello_world::HelloWorldTopic"),
        other => panic!("expected struct schema, got {other:?}"),
    }

    let greeting = fields
        .iter()
        .find(|field| field.name == "greeting")
        .expect("greeting field");
    assert_eq!(greeting.value, DynamicTypeSchema::String { bound: None });

    let attitude = fields
        .iter()
        .find(|field| field.name == "attitude")
        .expect("attitude field");
    assert_eq!(
        attitude.value,
        <Quaternion as DdsFieldMetadata>::field_schema().clone()
    );
}

#[test]
fn round_trips_hello_world_topic() {
    let original = sample();
    let dynamic = original.to_dynamic_value();

    match &dynamic {
        DynamicValue::Struct(fields) => {
            assert_eq!(
                fields.get("greeting"),
                Some(&DynamicValue::String("Hello, world!".to_string()))
            );
            match fields.get("attitude") {
                Some(DynamicValue::Struct(attitude)) => {
                    assert_eq!(attitude.get("x"), Some(&DynamicValue::Float64(0.25)));
                    assert_eq!(attitude.get("w"), Some(&DynamicValue::Float64(1.0)));
                }
                other => panic!("expected nested attitude struct, got {other:?}"),
            }
        }
        other => panic!("expected struct value, got {other:?}"),
    }

    assert_eq!(HelloWorldTopic::from_dynamic_value(&dynamic).unwrap(), original);
}

#[test]
fn gets_and_sets_topic_fields() {
    let mut topic = sample();

    assert_eq!(
        topic.get_field("greeting").unwrap(),
        DynamicValue::String("Hello, world!".to_string())
    );
    topic
        .set_field("greeting", DynamicValue::String("Hola, mundo!".to_string()))
        .unwrap();
    assert_eq!(topic.greeting, "Hola, mundo!");

    let attitude = Quaternion {
        x: 1.0,
        y: 0.0,
        z: 0.0,
        w: 0.0,
    };
    topic
        .set_field(
            "attitude",
            <Quaternion as DdsFieldMetadata>::to_dynamic_value(&attitude),
        )
        .unwrap();
    assert_eq!(topic.attitude, attitude);

    assert!(topic
        .set_field("greeting", DynamicValue::Int32(1))
        .is_err());
}

#[test]
fn rejects_missing_required_fields() {
    let mut dynamic = sample().to_dynamic_value();
    if let DynamicValue::Struct(fields) = &mut dynamic {
        fields.remove("greeting");
    } else {
        panic!("expected struct value");
    }

    assert!(HelloWorldTopic::from_dynamic_value(&dynamic).is_err());
}

#[test]
fn enumerates_and_round_trips_view_topic() {
    let schema = HelloWorldViewTopic::field_schema();
    let fields = struct_fields(schema);

    assert_eq!(
        HelloWorldViewTopic::field_names(),
        vec![
            "greeting".to_string(),
            "attitude".to_string(),
            "valid".to_string()
        ]
    );
    let valid = fields
        .iter()
        .find(|field| field.name == "valid")
        .expect("valid field");
    assert_eq!(
        valid.value,
        DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Boolean)
    );

    let original = HelloWorldViewTopic {
        greeting: "Hello, world!".to_string(),
        attitude: quaternion(),
        valid: true,
    };
    let dynamic = original.to_dynamic_value();
    assert_eq!(
        HelloWorldViewTopic::from_dynamic_value(&dynamic).unwrap(),
        original
    );
}
