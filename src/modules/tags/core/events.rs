pub mod v1 {
    pub mod tag_color_set;
    pub mod tag_created;
    pub mod tag_deleted;
    pub mod tag_description_set;
    pub mod tag_name_set;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum TagEvent {
    TagCreatedV1(v1::tag_created::TagCreatedV1),
    TagDeletedV1(v1::tag_deleted::TagDeletedV1),
    TagNameSetV1(v1::tag_name_set::TagNameSetV1),
    TagColorSetV1(v1::tag_color_set::TagColorSetV1),
    TagDescriptionSetV1(v1::tag_description_set::TagDescriptionSetV1),
}
