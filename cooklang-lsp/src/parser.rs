use core::fmt;

use std::fmt::{Debug, Formatter};
use tree_sitter::{Node, Parser};

pub(crate) struct CooklangParser {
    parser: Parser,
}

impl Debug for CooklangParser {
    fn fmt(&self, _: &mut Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct ParseResult {
    // pub(crate) tree: Tree, // tree need to be store if we want to benefit from iterative parsing
    pub(crate) ingredients: Vec<String>,
    pub(crate) cookwares: Vec<String>,
    pub(crate) step_lines: Vec<usize>,
}

/// extract the name of ingredient or cookware in the source code matching the node.
/// note that there can't be a comment_line in the name.
fn extract_clean_name(node: &Node, source_code: &str) -> String {
    let range = node.byte_range();
    let mut cursor = node.walk();
    let mut text = "".to_owned();
    let mut start_byte = range.start;
    for sub_node in node.children(&mut cursor) {
        if sub_node.kind() == "comment_block" {
            let range_sub_node = sub_node.byte_range();
            // add everything before comment to text
            text += &source_code[start_byte..range_sub_node.start];
            start_byte = range_sub_node.end;
        }
    }

    text += &source_code[start_byte..range.end];
    text
}

impl CooklangParser {
    pub(crate) fn new() -> CooklangParser {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_cooklang::language())
            .expect("Error loading Cooklang grammar");

        CooklangParser { parser }
    }

    pub(crate) fn parse(&mut self, source_code: &str) -> ParseResult {
        let tree = self.parser.parse(source_code, None).unwrap();

        let root_node = tree.root_node();
        let mut cursor = root_node.walk();
        let mut ingredients = vec![];
        let mut cookwares = vec![];
        let mut step_lines = vec![];
        root_node.children(&mut cursor).for_each(|node| {
            if node.kind() == "step" {
                step_lines.push(node.start_position().row);
                let mut cursor = node.walk();
                node.children(&mut cursor).for_each(|node| {
                    if node.kind() == "ingredient" {
                        let node = node.child_by_field_name("name").unwrap();
                        let name = extract_clean_name(&node, source_code);
                        ingredients.push(name);
                    }
                    if node.kind() == "cookware" {
                        let node = node.child_by_field_name("name").unwrap();
                        let name = extract_clean_name(&node, source_code);
                        cookwares.push(name);
                    }
                })
            }
        });

        ParseResult {
            // tree: tree.to_owned(),
            ingredients,
            cookwares,
            step_lines,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::CooklangParser;

    #[test]
    fn test_tricky() {
        let text = "
            foo @bar bar{1%kg}\n\
            >> foo[-foo--?-]:bar[- -- BAR -]\n\
            >> ba/r: fo-o\n\
            foo @po-- [--]mme verte--- {}\n\
            foo @po [--]mme verte{}\n\
            foo @po[--]mme verte{1}\n\
            foo @po [--] mme verte{1%kg}\n\
            foo @po[--] mme verte{1 %kg}\n\
            foo @po [--]mme verte{   1  %  kg }\n\
            foo @po [--]mme verte{   1 [- ou plus-] %  kg [- ou pas -] } dans une #poil\n\
            foo @po [--]mme verte{   1 [- ou plus-] %  kg [- ou pas -] }\n\
            >> ba/r: fo-o\n\
        ";

        let mut parser = CooklangParser::new();
        let parsed = parser.parse(text);
        println!("{parsed:#?}");
        assert_eq!(
            parsed.ingredients,
            vec![
                "bar bar",
                "po",
                "po mme verte",
                "pomme verte",
                "po  mme verte",
                "po mme verte",
                "po mme verte",
                "po mme verte",
                "po mme verte",
            ]
        );
        assert_eq!(parsed.cookwares, vec!["poil"]);
        assert_eq!(parsed.step_lines, vec![1, 4, 5, 6, 7, 8, 9, 10, 11,])
    }
}
