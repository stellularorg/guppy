use shared::markdown::parse_markdown as shared_parse_markdown;

pub fn render(original_in: &String) -> String {
    shared_parse_markdown(original_in.to_owned(), Vec::new())
}
