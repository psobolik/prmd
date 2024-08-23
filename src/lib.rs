use comrak::arena_tree::Node;
use comrak::nodes::{
    Ast, ListDelimType, ListType, NodeCodeBlock, NodeHeading, NodeHtmlBlock, NodeList, NodeTable,
    NodeValue, TableAlignment,
};
use comrak::{Arena, Options};
use std::cell::RefCell;

pub fn markdown_to_text(md: &str, plain: bool) -> String {
    let arena = Arena::new();
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    let root = comrak::parse_document(&arena, md, &options);
    ast_to_text(root, plain)
}

fn ast_to_text<'a>(root: &'a Node<'a, RefCell<Ast>>, plain: bool) -> String {
    fn node_children_to_formatted_text<'a>(
        node: &'a Node<'a, RefCell<Ast>>,
        plain: bool,
        bol: Option<&'a str>,
        eol: Option<&'a str>,
    ) -> String {
        node.children().fold(String::new(), |acc, child| {
            let eol = eol.unwrap_or_default();
            let bol = bol.unwrap_or_default();
            let text = format!("{}{}", acc, text_node_to_text(child, plain));
            format!("{}{}{}", bol, text, eol)
        })
    }
    fn node_children_to_plain_text<'a>(node: &'a Node<'a, RefCell<Ast>>) -> String {
        node_children_to_formatted_text(node, true, None, None)
    }
    fn text_node_to_text<'a>(text_node: &'a Node<'a, RefCell<Ast>>, plain: bool) -> String {
        match &text_node.data.borrow().value {
            NodeValue::Emph => {
                if plain {
                    node_children_to_plain_text(text_node)
                } else {
                    node_children_to_formatted_text(
                        text_node,
                        plain,
                        Some("\x1b[3m"),
                        Some("\x1b[0m"),
                    )
                }
            }
            NodeValue::Strong => {
                if plain {
                    node_children_to_plain_text(text_node)
                } else {
                    node_children_to_formatted_text(
                        text_node,
                        plain,
                        Some("\x1b[1m"),
                        Some("\x1b[0m"),
                    )
                }
            }
            NodeValue::Underline => {
                if plain {
                    node_children_to_plain_text(text_node)
                } else {
                    node_children_to_formatted_text(
                        text_node,
                        plain,
                        Some("\x1b[4m"),
                        Some("\x1b[0m"),
                    )
                }
            }
            NodeValue::Strikethrough => {
                if plain {
                    node_children_to_plain_text(text_node)
                } else {
                    node_children_to_formatted_text(
                        text_node,
                        plain,
                        Some("\x1b[9m"),
                        Some("\x1b[0m"),
                    )
                }
            }
            NodeValue::Code(code) => {
                if plain {
                    code.literal.clone()
                } else {
                    format!("\x1b[7m{}\x1b[0m", code.literal)
                }
            }
            NodeValue::Link(image) | NodeValue::Image(image) => {
                let url = format!("[{}]", image.url);
                let title = if image.title.len() > 0 {
                    format!(r#" "{}""#, image.title)
                } else {
                    String::from("")
                };
                let text = if plain {
                    node_children_to_plain_text(text_node)
                } else {
                    node_children_to_formatted_text(
                        text_node,
                        plain,
                        Some("\x1b[4m"),
                        Some("\x1b[0m"),
                    )
                };
                format!("{}{} {}", text, title, url)
            }
            NodeValue::Paragraph => paragraph_node_to_text(text_node, plain),
            NodeValue::SoftBreak => String::from(" "),
            NodeValue::LineBreak => String::from("\n"),
            NodeValue::HtmlInline(html_inline) => html_inline.clone(),
            NodeValue::Text(text) => text.clone(),
            _ => format!("💔 unexpected child in Text node: {:#?}", text_node),
        }
    }
    fn thematic_break_node_to_text() -> String {
        String::from("¶\n")
    }
    fn blockquote_node_to_text<'a>(
        blockquote_node: &'a Node<'a, RefCell<Ast>>,
        level: usize,
        plain: bool,
    ) -> String {
        let blockquote = blockquote_node
            .children()
            .fold(String::new(), |acc, child| {
                let lead = "│ ".repeat(level + 1);
                match child.data.borrow().value {
                    NodeValue::BlockQuote => {
                        format!(
                            "{}{}",
                            acc,
                            blockquote_node_to_text(child, level + 1, plain)
                        )
                    }
                    _ => format!(
                        "{}{}\n",
                        lead,
                        node_children_to_formatted_text(child, plain, None, None)
                    ),
                }
            });
        if level == 0 {
            format!("{}\n", blockquote)
        } else {
            blockquote
        }
    }
    fn code_block_node_to_text(code_block: &mut NodeCodeBlock, plain: bool) -> String {
        let lines = code_block.literal.lines().fold(String::new(), |acc, line| {
            if plain {
                format!("{}║ {}\n", acc, line)
            } else {
                format!("{}\x1b[97m\x1b[48;5;238m{}\x1b[0K\x1b[0m\n", acc, line)
            }
        });
        format!("{}\n", lines)
    }
    fn html_block_node_to_text(html_block_node: &mut NodeHtmlBlock, _plain: bool) -> String {
        // We don't try to parse HTML
        format!("{}\n", html_block_node.literal)
    }
    fn paragraph_node_to_text<'a>(
        paragraph_node: &'a Node<'a, RefCell<Ast>>,
        plain: bool,
    ) -> String {
        let paragraph = node_children_to_formatted_text(paragraph_node, plain, None, None);
        format!("{}\n", paragraph)
    }
    fn heading_node_to_text<'a>(
        node: &'a Node<'a, RefCell<Ast>>,
        heading: &mut NodeHeading,
        plain: bool,
    ) -> String {
        let ansi = if plain {
            ""
        } else {
            match heading.level {
                1 => "\x1b[1;4m", // Bold, underlined
                2 => "\x1b[1;3m", // Bold, italic
                3 => "\x1b[3;4m", // Italic, underlined
                4 => "\x1b[3m",   // Italic
                _ => "\x1b[4m",   // Underlined
            }
        };
        node_children_to_formatted_text(node, plain, Some(ansi), Some("\x1b[0m\n\n"))
    }
    fn table_node_to_text<'a>(
        table_node: &'a Node<'a, RefCell<Ast>>,
        node_table: &mut NodeTable,
        plain: bool,
    ) -> String {
        fn table_cell_node_to_text<'a>(
            table_cell_node: &'a Node<'a, RefCell<Ast>>,
            is_header: bool,
            width: usize,
            alignment: TableAlignment,
            plain: bool,
        ) -> String {
            let plain_content = node_children_to_plain_text(table_cell_node);
            let padding = width - plain_content.len();
            let (padding_left, padding_right) = match alignment {
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
            let content = if plain {
                plain_content
            } else if is_header {
                node_children_to_formatted_text(
                    table_cell_node,
                    plain,
                    Some("\x1b[1;4m"),
                    Some("\x1b[0m"),
                )
            } else {
                node_children_to_formatted_text(table_cell_node, plain, None, None)
            };
            format!("{}{} {}", padding_left, content, padding_right)
        }
        fn table_row_node_to_text<'a>(
            table_row_node: &'a Node<'a, RefCell<Ast>>,
            is_header: bool,
            column_widths: &[usize],
            alignments: &[TableAlignment],
            plain: bool,
        ) -> String {
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
                                        alignments[index],
                                        plain,
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
        let column_widths: Vec<Vec<usize>> = table_node
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
        let max_column_widths =
            column_widths
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
                            &max_column_widths,
                            &node_table.alignments,
                            plain,
                        )
                    )
                }
                _ => format!("{}💔 unexpected child in Table Node: {:#?}", acc, child),
            }
        });
        format!("\n{}\n", table)
    }
    fn list_node_to_text<'a>(
        list_node: &'a Node<'a, RefCell<Ast>>,
        level: usize,
        plain: bool,
    ) -> String {
        fn item_node_to_text<'a>(
            item_node: &'a Node<'a, RefCell<Ast>>,
            level: usize,
            node_list: &NodeList,
            plain: bool,
        ) -> String {
            item_node.children().fold(String::new(), |acc, child| {
                match child.data.borrow().value {
                    NodeValue::List(_node_list) => {
                        format!("{}{}", acc, list_node_to_text(child, level + 1, plain))
                    }
                    NodeValue::Paragraph => {
                        // • ▪ ◦
                        let (marker, marker_len) = if node_list.list_type == ListType::Bullet {
                            (
                                match level {
                                    0 => "•",
                                    1 => "◦",
                                    _ => "▪",
                                }
                                .to_string(),
                                1,
                            )
                        } else {
                            let delimiter = match node_list.delimiter {
                                ListDelimType::Period => '.',
                                ListDelimType::Paren => ')',
                            };
                            (format!("{}{}", node_list.start, delimiter), 2)
                        };
                        let lead = " ".repeat(level * 4);
                        let lead2 = " ".repeat(marker_len + 1);
                        let text = paragraph_node_to_text(child, plain)
                            .lines()
                            .enumerate()
                            .fold(String::new(), |acc, (index, line)| {
                                let cooked = match index {
                                    0 => format!("{}{} {}\n", lead, marker, line),
                                    _ => format!("{} {}{}\n", lead, lead2, line),
                                };
                                format!("{}{}", acc, cooked)
                            });
                        format!("{} {}", acc, text)
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
                        item_node_to_text(child, level, &item_node_list, plain)
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

    let mut document: Vec<String> = vec![];
    root.children()
        .for_each(|child| match &mut child.data.borrow_mut().value {
            NodeValue::Paragraph => {
                document.push(paragraph_node_to_text(child, plain));
            }
            NodeValue::List(_node_list) => {
                document.push(list_node_to_text(child, 0, plain));
            }
            NodeValue::Heading(heading) => {
                document.push(heading_node_to_text(child, heading, plain));
            }
            NodeValue::CodeBlock(code_block) => {
                document.push(code_block_node_to_text(code_block, plain));
            }
            NodeValue::ThematicBreak => {
                document.push(thematic_break_node_to_text());
            }
            NodeValue::BlockQuote => {
                document.push(blockquote_node_to_text(child, 0, plain));
            }
            NodeValue::HtmlBlock(html_block) => {
                document.push(html_block_node_to_text(html_block, plain));
            }
            NodeValue::Table(node_table) => {
                document.push(table_node_to_text(child, node_table, plain));
            }
            _ => eprintln!("💔 Unexpected node in Document: {:#?}", child),
        });
    document.join("")
}
