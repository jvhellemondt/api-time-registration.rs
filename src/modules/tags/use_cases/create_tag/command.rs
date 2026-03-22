pub const PASTEL_COLORS: [&str; 10] = [
    "#FFB3BA", // pastel pink
    "#FFDFBA", // pastel orange
    "#FFFFBA", // pastel yellow
    "#BAFFC9", // pastel green
    "#BAE1FF", // pastel blue
    "#D4BAFF", // pastel purple
    "#FFBAF3", // pastel magenta
    "#BAF7FF", // pastel cyan
    "#FFC8BA", // pastel coral
    "#BAFFED", // pastel mint
];

pub fn pick_pastel_color() -> &'static str {
    let idx = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as usize % PASTEL_COLORS.len();
    PASTEL_COLORS[idx]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTag {
    pub tag_id: String,
    pub tenant_id: String,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub created_by: String,
}

#[cfg(test)]
mod create_tag_command_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn pick_pastel_color_returns_one_of_ten_pastels() {
        let color = pick_pastel_color();
        assert!(PASTEL_COLORS.contains(&color));
    }

    #[rstest]
    fn create_tag_holds_fields() {
        let cmd = CreateTag {
            tag_id: "t1".to_string(),
            tenant_id: "ten1".to_string(),
            name: "Work".to_string(),
            color: "#FFB3BA".to_string(),
            description: None,
            created_at: 1000,
            created_by: "u1".to_string(),
        };
        assert_eq!(cmd.tag_id, "t1");
        assert_eq!(cmd.name, "Work");
        assert_eq!(cmd.description, None);
    }
}
