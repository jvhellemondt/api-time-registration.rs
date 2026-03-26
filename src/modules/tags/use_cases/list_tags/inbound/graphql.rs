use async_graphql::{Context, Object, Result as GqlResult};

use crate::modules::tags::use_cases::list_tags::projection::TagView;
use crate::shared::infrastructure::request_context::RequestContext;
use crate::shell::state::AppState;

#[cfg(test)]
mod list_tags_graphql_inbound_tests {
    use async_graphql::{EmptySubscription, Schema};

    use super::{GqlTag, TagView};
    use crate::shared::infrastructure::request_context::RequestContext;
    use crate::shell::graphql::{MutationRoot, QueryRoot};
    use crate::tests::fixtures::tags::make_test_app_state;

    fn make_schema_from_state(
        state: crate::shell::state::AppState,
    ) -> Schema<QueryRoot, MutationRoot, EmptySubscription> {
        Schema::build(
            QueryRoot::default(),
            MutationRoot::default(),
            EmptySubscription,
        )
        .data(state)
        .finish()
    }

    fn req_ctx() -> RequestContext {
        RequestContext {
            user_id: "u-1".to_string(),
            tenant_id: "tenant-test".to_string(),
        }
    }

    #[tokio::test]
    async fn resolver_returns_empty_list_when_no_tags() {
        let schema = make_schema_from_state(make_test_app_state());
        let result = schema
            .execute(async_graphql::Request::new(r#"{ listTags { tagId } }"#).data(req_ctx()))
            .await;
        assert!(result.errors.is_empty());
        assert_eq!(result.data.to_string(), "{listTags: []}");
    }

    #[test]
    fn gql_tag_from_tag_view_maps_all_fields() {
        let view = TagView {
            tag_id: "tag-0001".to_string(),
            name: "my-tag".to_string(),
            color: "#abcdef".to_string(),
            description: Some("a desc".to_string()),
            deleted: false,
        };
        let gql = GqlTag::from(view);
        assert_eq!(gql.tag_id, "tag-0001");
        assert_eq!(gql.name, "my-tag");
        assert_eq!(gql.color, "#abcdef");
        assert_eq!(gql.description, Some("a desc".to_string()));
    }

    #[test]
    fn gql_tag_from_tag_view_handles_no_description() {
        let view = TagView {
            tag_id: "tag-0002".to_string(),
            name: "no-desc".to_string(),
            color: "#ffffff".to_string(),
            description: None,
            deleted: false,
        };
        let gql = GqlTag::from(view);
        assert_eq!(gql.description, None);
    }
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct GqlTag {
    pub tag_id: String,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
}

impl From<TagView> for GqlTag {
    fn from(v: TagView) -> Self {
        Self {
            tag_id: v.tag_id,
            name: v.name,
            color: v.color,
            description: v.description,
        }
    }
}

#[derive(Default)]
pub struct ListTagsQuery;

#[Object]
impl ListTagsQuery {
    async fn list_tags(&self, context: &Context<'_>) -> GqlResult<Vec<GqlTag>> {
        context
            .data::<RequestContext>()
            .map_err(|_| async_graphql::Error::new("Unauthorized"))?;
        let state = context.data_unchecked::<AppState>();
        let tags: Vec<TagView> = state.list_tags_handler.list_all().await?;
        Ok(tags.into_iter().map(Into::into).collect())
    }
}
