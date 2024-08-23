use comrak::arena_tree::Node;
use comrak::nodes::{
    Ast, ListDelimType, ListType, NodeCodeBlock, NodeHeading, NodeHtmlBlock, NodeList, NodeTable,
    NodeValue, TableAlignment,
};
use comrak::{Arena, Options};
use std::cell::RefCell;

pub fn markdown_to_text(md: &str) -> String {
    let arena = Arena::new();
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    let root = comrak::parse_document(&arena, md, &options);
    ast_to_text(root)
}

fn ast_to_text<'a>(root: &'a Node<'a, RefCell<Ast>>) -> String {
    fn node_children_to_plain_text<'a>(node: &'a Node<'a, RefCell<Ast>>) -> String {
        node.children().fold(String::new(), |acc, child| {
            format!("{}{}", acc, text_node_to_plain_text(child))
        })
    }
    fn text_node_to_plain_text<'a>(text_node: &'a Node<'a, RefCell<Ast>>) -> String {
        match &text_node.data.borrow().value {
            NodeValue::Emph
            | NodeValue::Strong
            | NodeValue::Underline
            | NodeValue::Strikethrough
            | NodeValue::Paragraph => node_children_to_plain_text(text_node),
            NodeValue::SoftBreak => String::from(" "),
            NodeValue::LineBreak => String::from("\n"),
            NodeValue::Code(code) => code.literal.clone(),
            NodeValue::HtmlInline(html_inline) => html_inline.clone(),
            NodeValue::Link(link) => {
                format!("{} [{}]", node_children_to_plain_text(text_node), link.url)
            }
            NodeValue::Image(image) => {
                format!("{} {}", node_children_to_plain_text(text_node), image.url)
            }
            NodeValue::Text(text) => text.clone(),
            _ => format!("💔 unexpected child in Text node: {:#?}", text_node),
        }
    }
    fn node_children_to_text<'a>(
        node: &'a Node<'a, RefCell<Ast>>,
        bol: Option<&'a str>,
        eol: Option<&'a str>,
    ) -> String {
        node.children().fold(String::new(), |acc, child| {
            let eol = eol.unwrap_or_default();
            let bol = bol.unwrap_or_default();
            let text = format!("{}{}", acc, text_node_to_formatted_text(child));
            format!("{}{}{}", bol, text, eol)
        })
    }
    fn text_node_to_formatted_text<'a>(text_node: &'a Node<'a, RefCell<Ast>>) -> String {
        match &text_node.data.borrow().value {
            NodeValue::Emph => node_children_to_text(text_node, Some("\x1b[3m"), Some("\x1b[0m")),
            NodeValue::Strong => node_children_to_text(text_node, Some("\x1b[1m"), Some("\x1b[0m")),
            NodeValue::Underline => {
                node_children_to_text(text_node, Some("\x1b[4m"), Some("\x1b[0m"))
            }
            NodeValue::Strikethrough => {
                node_children_to_text(text_node, Some("\x1b[9m"), Some("\x1b[0m"))
            }
            NodeValue::SoftBreak => String::from(" "),
            NodeValue::Paragraph => paragraph_node_to_text(text_node),
            NodeValue::LineBreak => String::from("\n"),
            NodeValue::Code(code) => format!("\x1b[7m{}\x1b[0m", code.literal),
            NodeValue::HtmlInline(html_inline) => html_inline.clone(),
            NodeValue::Link(link) => format!(
                "{} [{}]",
                node_children_to_text(text_node, Some("\x1b[4m"), Some("\x1b[0m")),
                link.url
            ),
            NodeValue::Image(image) => format!(
                "{} [\x1b[4m{}\x1b[0m]",
                node_children_to_text(text_node, Some("\x1b[1m"), Some("\x1b[0m")),
                image.url
            ),
            NodeValue::Text(text) => text.clone(),
            _ => format!("💔 unexpected child in Text node: {:#?}", text_node),
        }
    }
    fn thematic_break_node_to_text() -> String {
        String::from("¶\n\n")
    }
    fn blockquote_node_to_text<'a>(
        blockquote_node: &'a Node<'a, RefCell<Ast>>,
        level: usize,
    ) -> String {
        let blockquote = blockquote_node
            .children()
            .fold(String::new(), |acc, child| {
                let lead = "│ ".repeat(level + 1);
                match child.data.borrow().value {
                    NodeValue::BlockQuote => {
                        format!("{}{}", acc, blockquote_node_to_text(child, level + 1))
                    }
                    _ => format!("{}{}", lead, node_children_to_text(child, None, None)),
                }
            });
        format!("\n{}\n", blockquote)
    }
    fn code_block_node_to_text(code_block: &mut NodeCodeBlock) -> String {
        let lines = code_block.literal.lines().fold(String::new(), |acc, line| {
            format!("{}\x1b[97m\x1b[48;5;238m{}\x1b[0K\x1b[0m\n", acc, line)
        });
        format!("{}\n", lines)
    }
    fn html_block_node_to_text(html_block_node: &mut NodeHtmlBlock) -> String {
        // We don't try to parse HTML
        format!("{}\n", html_block_node.literal)
    }
    fn paragraph_node_to_text<'a>(paragraph_node: &'a Node<'a, RefCell<Ast>>) -> String {
        let paragraph = node_children_to_text(paragraph_node, None, None);
        format!("{}\n", paragraph)
    }
    fn heading_node_to_text<'a>(
        node: &'a Node<'a, RefCell<Ast>>,
        heading: &mut NodeHeading,
    ) -> String {
        let ansi = match heading.level {
            1 => "\x1b[1;4m", // Bold, underlined
            2 => "\x1b[1;3m", // Bold, italic
            3 => "\x1b[3;4m", // Italic, underlined
            4 => "\x1b[3m",   // Italic
            _ => "\x1b[4m",   // Underlined
        };
        node_children_to_text(node, Some(ansi), Some("\x1b[0m\n\n"))
    }
    fn table_node_to_text<'a>(
        table_node: &'a Node<'a, RefCell<Ast>>,
        node_table: &mut NodeTable,
    ) -> String {
        fn table_cell_node_to_text<'a>(
            table_cell_node: &'a Node<'a, RefCell<Ast>>,
            is_header: bool,
            width: usize,
            alignment: TableAlignment,
        ) -> String {
            let text = node_children_to_plain_text(table_cell_node);
            let padding = width - text.len();
            let (lead, trail) = match alignment {
                TableAlignment::Center => {
                    let left_padding = padding / 2;
                    let right_padding = padding - left_padding;
                    (" ".repeat(left_padding), " ".repeat(right_padding))
                }
                TableAlignment::Right => (" ".repeat(padding), String::default()),
                TableAlignment::None | TableAlignment::Left => {
                    (String::default(), " ".repeat(padding))
                }
            };
            if is_header {
                // node_children_to_text(table_cell_node, Some("\x1b[1m"), Some("\x1b[0m"))
                let text = node_children_to_text(table_cell_node, None, None);
                format!("\x1b[1;4m{}{} {}\x1b[0K\x1b[0m", lead, text, trail)
            } else {
                let text = node_children_to_text(table_cell_node, None, None);
                format!("{}{} {}", lead, text, trail)
            }
        }
        fn table_row_node_to_text<'a>(
            table_row_node: &'a Node<'a, RefCell<Ast>>,
            is_header: bool,
            column_widths: &Vec<usize>,
            alignments: &[TableAlignment],
        ) -> String {
            println!("column lengths: {:?}", column_widths);
            let row =
                table_row_node
                    .children()
                    .enumerate()
                    .fold(String::new(), |acc, (index, child)| {
                        match child.data.borrow().value {
                            NodeValue::TableCell => {
                                format!(
                                    "{}{}",
                                    acc,
                                    table_cell_node_to_text(
                                        child,
                                        is_header,
                                        column_widths[index],
                                        alignments[index]
                                    )
                                )
                            }
                            _ => format!(
                                "{}💔 unexpected child in Table Row Node: {:#?}",
                                acc, child
                            ),
                        }
                    });
            format!("{}\n", row)
        }
        let row_column_widths: Vec<Vec<usize>> = table_node
            .children()
            .map(|row| match row.data.borrow().value {
                NodeValue::TableRow(_is_header) => row
                    .children()
                    .map(|cell| match cell.data.borrow().value {
                        NodeValue::TableCell => node_children_to_plain_text(cell).len(),
                        _ => 0,
                    })
                    .collect(),
                _ => vec![],
            })
            .collect();
        let column_widths =
            row_column_widths
                .iter()
                .fold(vec![0; node_table.num_columns], |mut acc, row| {
                    for i in 0..node_table.num_columns {
                        if row[i] > acc[i] {
                            acc[i] = row[i];
                        }
                    }
                    acc
                });
        let table = table_node.children().fold(String::new(), |acc, child| {
            match child.data.borrow().value {
                NodeValue::TableRow(is_header) => {
                    format!(
                        "{}{}",
                        acc,
                        table_row_node_to_text(
                            child,
                            is_header,
                            &column_widths,
                            &node_table.alignments
                        )
                    )
                }
                _ => format!("{}💔 unexpected child in Table Node: {:#?}", acc, child),
            }
        });
        format!("\n{}\n", table)
    }
    fn list_node_to_text<'a>(list_node: &'a Node<'a, RefCell<Ast>>, level: usize) -> String {
        fn item_node_to_text<'a>(
            item_node: &'a Node<'a, RefCell<Ast>>,
            level: usize,
            node_list: &NodeList,
        ) -> String {
            item_node.children().fold(String::new(), |acc, child| {
                match child.data.borrow().value {
                    NodeValue::List(_node_list) => {
                        format!("{}{}", acc, list_node_to_text(child, level + 1))
                    }
                    NodeValue::Paragraph => {
                        // • ▪ ◦
                        let (marker, marker_len) = if node_list.list_type == ListType::Bullet {
                            (match level {
                                0 => "•",
                                1 => "◦",
                                _ => "▪",
                            }
                            .to_string(), 1)
                        } else {
                            let delimiter = match node_list.delimiter {
                                ListDelimType::Period => '.',
                                ListDelimType::Paren => ')',
                            };
                            (format!("{}{}", node_list.start, delimiter), 2)
                        };
                        let lead = " ".repeat(level * 4);
                        let lead2 = " ".repeat(marker_len + 1);
                        let text = paragraph_node_to_text(child).lines().enumerate().fold(String::new(), |acc, (index, line)| {
                            let cooked = match index {
                                0 => format!("{}{} {}\n", lead, marker, line),
                                _ => format!("{} {}{}\n", lead, lead2, line),
                            };
                            format!("{}{}", acc, cooked)
                        });
                        format!("{} {}", acc, text)
                        // format!("{}{}{} {}", acc, lead, char, paragraph_node_to_text(child))
                    }
                    _ => format!("{}💔 unexpected child in List Item Node: {:#?}", acc, child),
                }
            })
        }
        let items = list_node.children().fold(String::new(), |acc, child| {
            match child.data.borrow().value {
                NodeValue::Item(item_node_list) => {
                    format!(
                        "{}{}",
                        acc,
                        item_node_to_text(child, level, &item_node_list)
                    )
                }
                _ => format!("{}💔 unexpected child in List Node: {:#?}", acc, child),
            }
        });
        if level == 0 {
            format!("{}\n", items)
        } else {
            items
        }
    }
    fn document_node_to_text<'a>(document: &'a Node<'a, RefCell<Ast>>) -> String {
        let mut document_string = String::new();

        document.children().for_each(|child| {
            match &mut child.data.borrow_mut().value {
                NodeValue::Paragraph => {
                    document_string += paragraph_node_to_text(child).as_str();
                }
                NodeValue::List(_node_list) => {
                    document_string += list_node_to_text(child, 0).as_str();
                }
                NodeValue::Heading(heading) => {
                    document_string += heading_node_to_text(child, heading).as_str();
                }
                NodeValue::CodeBlock(code_block) => {
                    document_string += code_block_node_to_text(code_block).as_str();
                }
                NodeValue::ThematicBreak => {
                    document_string += thematic_break_node_to_text().as_str();
                }
                NodeValue::BlockQuote => {
                    document_string += blockquote_node_to_text(child, 0).as_str();
                }
                NodeValue::HtmlBlock(html_block) => {
                    document_string += html_block_node_to_text(html_block).as_str();
                }
                NodeValue::Table(node_table) => {
                    document_string += table_node_to_text(child, node_table).as_str();
                }
                NodeValue::DescriptionList => {
                    println!("🔴 DescriptionList♦");
                }
                NodeValue::DescriptionItem(description_item) => {
                    println!("🔴 DescriptionItem: {:#?}♦", description_item);
                }
                NodeValue::DescriptionTerm => {
                    println!("🔴 Description Term♦");
                }
                NodeValue::DescriptionDetails => {
                    println!("🔴 Description Details♦");
                }
                NodeValue::FootnoteDefinition(footnote_definition) => {
                    println!("🔴 FootnoteDefinition: {:#?}♦", footnote_definition);
                }
                NodeValue::TableRow(is_header) => {
                    println!("🔴 TableRow: {:#?}♦", is_header);
                }
                NodeValue::TableCell => {
                    println!("🔴 TableCell♦");
                }
                NodeValue::Text(text) => {
                    println!("🔴 Text: {text}♦")
                }
                NodeValue::TaskItem(check_char) => {
                    println!("🔴 TaskItem: {:#?}♦", check_char);
                }
                NodeValue::SoftBreak => {
                    println!("🔴 SoftBreak♦");
                }
                NodeValue::LineBreak => {
                    println!("🔴 LineBreak♦");
                }
                NodeValue::Code(code) => {
                    println!("🔴 Code: {:#?}♦", code);
                }
                NodeValue::HtmlInline(html_inline) => {
                    println!("🔴 HtmlInline: {:#?}♦", html_inline);
                }
                NodeValue::Emph => {
                    println!("🔴 Emph♦");
                }
                NodeValue::Strong => {
                    println!("🔴 Strong♦");
                }
                NodeValue::Strikethrough => {
                    println!("🔴 Strikethrough♦");
                }
                NodeValue::Superscript => {
                    println!("🔴 Superscript♦");
                }
                NodeValue::Link(link) => {
                    println!("🔴 Link: {:#?}♦", link);
                }
                NodeValue::Image(image) => {
                    println!("🔴 Image: {:#?}♦", image);
                }
                NodeValue::FootnoteReference(footnote_reference) => {
                    println!("🔴 FootnoteReference: {:#?}♦", footnote_reference);
                }
                // NodeValue::ShortCode(shortCode) => {
                //     println!("🔴 ShortCode: {:#?}♦", shortCode);
                // }
                NodeValue::Math(math) => {
                    println!("🔴 Math: {:#?}♦", math);
                }
                NodeValue::MultilineBlockQuote(multiline_block_quote) => {
                    println!("🔴 MultilineBlockQuote: {:#?}♦", multiline_block_quote);
                }
                NodeValue::Escaped => {
                    println!("🔴 Escaped♦");
                }
                NodeValue::WikiLink(wiki_link) => {
                    println!("🔴 WikiLink: {:#?}♦", wiki_link);
                }
                NodeValue::Underline => {
                    println!("🔴 Underline♦");
                }
                NodeValue::SpoileredText => {
                    println!("🔴 SpoileredText♦");
                }
                NodeValue::EscapedTag(escaped_tag) => {
                    println!("🔴 EscapedTag: {:#?}♦", escaped_tag);
                }
                _ => {}
            }
        });
        document_string
    }
    document_node_to_text(root)
}
