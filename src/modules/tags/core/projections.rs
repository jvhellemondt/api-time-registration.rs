use crate::modules::tags::core::events::TagEvent;
use crate::modules::tags::use_cases::list_tags::projection::TagRow;

pub enum Mutation {
    Upsert(TagRow),
    MarkDeleted {
        tag_id: String,
        deleted_at: i64,
        deleted_by: String,
        last_event_id: String,
    },
    SetName {
        tag_id: String,
        name: String,
        last_event_id: String,
    },
    SetColor {
        tag_id: String,
        color: String,
        last_event_id: String,
    },
    SetDescription {
        tag_id: String,
        description: Option<String>,
        last_event_id: String,
    },
}

pub fn apply(stream_id: &str, version: i64, event: &TagEvent) -> Vec<Mutation> {
    let stream_key = format!("{stream_id}:{version}");
    match event {
        TagEvent::TagCreatedV1(e) => vec![Mutation::Upsert(TagRow {
            tag_id: e.tag_id.clone(),
            tenant_id: e.tenant_id.clone(),
            name: e.name.clone(),
            color: e.color.clone(),
            description: e.description.clone(),
            deleted: false,
            last_event_id: Some(stream_key),
        })],
        TagEvent::TagDeletedV1(e) => vec![Mutation::MarkDeleted {
            tag_id: e.tag_id.clone(),
            deleted_at: e.deleted_at,
            deleted_by: e.deleted_by.clone(),
            last_event_id: stream_key,
        }],
        TagEvent::TagNameSetV1(e) => vec![Mutation::SetName {
            tag_id: e.tag_id.clone(),
            name: e.name.clone(),
            last_event_id: stream_key,
        }],
        TagEvent::TagColorSetV1(e) => vec![Mutation::SetColor {
            tag_id: e.tag_id.clone(),
            color: e.color.clone(),
            last_event_id: stream_key,
        }],
        TagEvent::TagDescriptionSetV1(e) => vec![Mutation::SetDescription {
            tag_id: e.tag_id.clone(),
            description: e.description.clone(),
            last_event_id: stream_key,
        }],
    }
}

#[cfg(test)]
mod tag_projections_apply_tests {
    use super::*;
    use crate::modules::tags::core::events::v1::tag_color_set::TagColorSetV1;
    use crate::modules::tags::core::events::v1::tag_created::TagCreatedV1;
    use crate::modules::tags::core::events::v1::tag_deleted::TagDeletedV1;
    use crate::modules::tags::core::events::v1::tag_description_set::TagDescriptionSetV1;
    use crate::modules::tags::core::events::v1::tag_name_set::TagNameSetV1;
    use rstest::rstest;

    fn make_created() -> TagEvent {
        TagEvent::TagCreatedV1(TagCreatedV1 {
            tag_id: "t1".to_string(),
            tenant_id: "ten1".to_string(),
            name: "Work".to_string(),
            color: "#FFB3BA".to_string(),
            description: None,
            created_at: 1000,
            created_by: "u1".to_string(),
        })
    }

    #[rstest]
    fn it_applies_tag_created() {
        let mutations = apply("Tag-t1", 1, &make_created());
        assert_eq!(mutations.len(), 1);
        assert!(matches!(&mutations[0], Mutation::Upsert(_)));
    }

    #[rstest]
    fn it_applies_tag_deleted() {
        let mutations = apply(
            "Tag-t1",
            2,
            &TagEvent::TagDeletedV1(TagDeletedV1 {
                tag_id: "t1".to_string(),
                tenant_id: "ten1".to_string(),
                deleted_at: 2000,
                deleted_by: "u1".to_string(),
            }),
        );
        assert_eq!(mutations.len(), 1);
        assert!(matches!(&mutations[0], Mutation::MarkDeleted { .. }));
    }

    #[rstest]
    fn it_applies_tag_name_set() {
        let mutations = apply(
            "Tag-t1",
            2,
            &TagEvent::TagNameSetV1(TagNameSetV1 {
                tag_id: "t1".to_string(),
                tenant_id: "ten1".to_string(),
                name: "Billable".to_string(),
                set_at: 2000,
                set_by: "u1".to_string(),
            }),
        );
        assert_eq!(mutations.len(), 1);
        assert!(matches!(&mutations[0], Mutation::SetName { .. }));
    }

    #[rstest]
    fn it_applies_tag_color_set() {
        let mutations = apply(
            "Tag-t1",
            2,
            &TagEvent::TagColorSetV1(TagColorSetV1 {
                tag_id: "t1".to_string(),
                tenant_id: "ten1".to_string(),
                color: "#BAE1FF".to_string(),
                set_at: 2000,
                set_by: "u1".to_string(),
            }),
        );
        assert_eq!(mutations.len(), 1);
        assert!(matches!(&mutations[0], Mutation::SetColor { .. }));
    }

    #[rstest]
    fn it_applies_tag_description_set() {
        let mutations = apply(
            "Tag-t1",
            2,
            &TagEvent::TagDescriptionSetV1(TagDescriptionSetV1 {
                tag_id: "t1".to_string(),
                tenant_id: "ten1".to_string(),
                description: Some("desc".to_string()),
                set_at: 2000,
                set_by: "u1".to_string(),
            }),
        );
        assert_eq!(mutations.len(), 1);
        assert!(matches!(&mutations[0], Mutation::SetDescription { .. }));
    }
}
