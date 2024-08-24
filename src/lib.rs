use ansi_term::{ANSIString, Color, Style};
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
    fn node_children_to_text<'a>(node: &'a Node<'a, RefCell<Ast>>, plain: bool) -> String {
        node.children()
            .map(|child| text_node_to_text(child, plain))
            .collect::<Vec<String>>()
            .join("")
    }
    fn text_node_to_text<'a>(text_node: &'a Node<'a, RefCell<Ast>>, plain: bool) -> String {
        match &text_node.data.borrow().value {
            NodeValue::Emph => {
                let text = node_children_to_text(text_node, plain);
                if plain {
                    text
                } else {
                    Style::new().italic().paint(text).to_string()
                }
            }
            NodeValue::Strong => {
                let text = node_children_to_text(text_node, plain);
                if plain {
                    text
                } else {
                    Style::new().bold().paint(text).to_string()
                }
            }
            NodeValue::Underline => {
                let text = node_children_to_text(text_node, plain);
                if plain {
                    text
                } else {
                    Style::new().underline().paint(text).to_string()
                }
            }
            NodeValue::Strikethrough => {
                let text = node_children_to_text(text_node, plain);
                if plain {
                    text
                } else {
                    Style::new().strikethrough().paint(text).to_string()
                }
            }
            NodeValue::Code(code) => {
                if plain {
                    code.literal.to_string()
                } else {
                    Style::new()
                        .fg(Color::White)
                        .bold()
                        .on(Color::Fixed(238))
                        .paint(&code.literal)
                        .to_string()
                }
            }
            NodeValue::Link(image) | NodeValue::Image(image) => {
                let title = if !image.title.is_empty() {
                    format!(r#" "{}""#, image.title)
                } else {
                    String::from("")
                };
                let text = node_children_to_text(text_node, plain);
                let content = if plain {
                    text.to_string()
                } else {
                    Style::new().underline().paint(text).to_string()
                };
                format!("{}{} [{}]", content, title, image.url)
            }
            NodeValue::Paragraph => paragraph_node_to_text(text_node, plain),
            NodeValue::SoftBreak => String::from(" "),
            NodeValue::LineBreak => String::from("\n"),
            NodeValue::HtmlInline(html_inline) => html_inline.clone(),
            NodeValue::Text(text) => text.clone(),
            _ => {
                eprintln!("💔 Unexpected child in Text node: {:#?}", text_node);
                "💔 Unexpected child in Text node".to_string()
            }
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
            .map(|child| match child.data.borrow().value {
                NodeValue::BlockQuote => blockquote_node_to_text(child, level + 1, plain),
                _ => {
                    let lead = "│ ".repeat(level + 1);
                    format!("{}{}\n", lead, node_children_to_text(child, plain))
                }
            })
            .collect();
        match level {
            0 => format!("{}\n", blockquote),
            _ => blockquote,
        }
    }
    fn code_block_node_to_text(code_block: &mut NodeCodeBlock, plain: bool) -> String {
        let info = if code_block.info.is_empty() {
            String::default()
        } else {
            let info = format!("[{}]\n", code_block.info);
            if plain {
                info
            } else {
                Style::new().reverse().paint(info).to_string()
            }
        };
        let lines: Vec<String> = code_block
            .literal
            .lines()
            .map(|line| {
                if plain {
                    format!("║ {}", line)
                } else {
                    let fancy_line = Style::new()
                        .fg(Color::White)
                        .bold()
                        .on(Color::Fixed(238))
                        .paint(format!("{}{}", line, ansi_escapes::EraseEndLine));
                    fancy_line.to_string()
                }
            })
            .collect();
        format!("{}{}\n\n", info, lines.join("\n"))
    }
    fn html_block_node_to_text(html_block_node: &mut NodeHtmlBlock, _plain: bool) -> String {
        // We don't try to parse HTML
        format!("{}\n", html_block_node.literal)
    }
    fn paragraph_node_to_text<'a>(
        paragraph_node: &'a Node<'a, RefCell<Ast>>,
        plain: bool,
    ) -> String {
        let paragraph = node_children_to_text(paragraph_node, plain);
        format!("{}\n\n", paragraph)
    }
    fn heading_node_to_text<'a>(
        node: &'a Node<'a, RefCell<Ast>>,
        heading: &mut NodeHeading,
        plain: bool,
    ) -> String {
        let text = node_children_to_text(node, plain);
        let heading_text = if plain {
            ANSIString::from(&text)
        } else {
            match heading.level {
                1 => Style::new().bold().underline().paint(&text),
                2 => Style::new().bold().italic().paint(&text),
                3 => Style::new().italic().underline().paint(&text),
                4 => Style::new().underline().paint(&text),
                _ => Style::new().italic().paint(&text),
            }
        };
        format!("{}\n\n", heading_text)
    }
    fn table_node_to_text<'a>(
        table_node: &'a Node<'a, RefCell<Ast>>,
        node_table: &mut NodeTable,
        plain: bool,
    ) -> String {
        fn max_column_widths<'a>(
            table_node: &'a Node<'a, RefCell<Ast>>,
            node_table: &mut NodeTable,
        ) -> Vec<usize> {
            let column_widths: Vec<Vec<usize>> = table_node
                .children()
                .map(|row| match row.data.borrow().value {
                    NodeValue::TableRow(_is_header) => row
                        .children()
                        .map(|cell| match cell.data.borrow().value {
                            NodeValue::TableCell => node_children_to_text(cell, true).len(),
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
            max_column_widths
        }
        fn table_cell_node_to_text<'a>(
            table_cell_node: &'a Node<'a, RefCell<Ast>>,
            is_header: bool,
            width: usize,
            alignment: TableAlignment,
            plain: bool,
        ) -> String {
            let plain_content = node_children_to_text(table_cell_node, true);
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
            let text = node_children_to_text(table_cell_node, plain);
            let content = if is_header && !plain {
                Style::new().bold().underline().paint(&text)
            } else {
                ANSIString::from(text)
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
            let row: Vec<String> = table_row_node
                .children()
                .enumerate()
                .map(|(index, child)| match child.data.borrow().value {
                    NodeValue::TableCell => table_cell_node_to_text(
                        child,
                        is_header,
                        column_widths[index],
                        alignments[index],
                        plain,
                    ),
                    _ => {
                        eprintln!("💔 Unexpected child in Table Row node: {:#?}", child);
                        "💔 Unexpected child in Table Row node".to_string()
                    }
                })
                .collect();
            format!("{}\n", row.join(""))
        }
        let max_column_widths = max_column_widths(table_node, node_table);
        let table: Vec<String> = table_node
            .children()
            .map(|child| match child.data.borrow().value {
                NodeValue::TableRow(is_header) => table_row_node_to_text(
                    child,
                    is_header,
                    &max_column_widths,
                    &node_table.alignments,
                    plain,
                ),
                _ => {
                    eprintln!("💔 Unexpected child in Table node: {:#?}", child);
                    "💔 Unexpected child in Table node".to_string()
                }
            })
            .collect();
        format!("{}\n", table.join(""))
    }
    fn list_node_to_text<'a>(
        list_node: &'a Node<'a, RefCell<Ast>>,
        level: usize,
        plain: bool,
    ) -> String {
        fn item_node_to_text<'a>(
            item_node: &'a Node<'a, RefCell<Ast>>,
            index: usize,
            level: usize,
            node_list: &NodeList,
            plain: bool,
        ) -> String {
            item_node
                .children()
                .map(|child| match child.data.borrow().value {
                    NodeValue::List(_node_list) => list_node_to_text(child, level + 1, plain),
                    NodeValue::Paragraph => {
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
                            (format!("{}{}", index + 1, delimiter), 2)
                        };
                        let indent = " ".repeat(level * 4);
                        let marker_space = " ".repeat(marker_len);
                        node_children_to_text(child, plain)
                            .lines()
                            .enumerate()
                            .map(|(index, line)| match index {
                                0 => format!("{}{} {}\n", indent, marker, line),
                                _ => format!("{}{} {}\n", indent, marker_space, line),
                            })
                            .collect()
                    }
                    _ => {
                        eprintln!("💔 Unexpected child in List Item node: {:#?}", child);
                        "💔 Unexpected child in List Item node".to_string()
                    }
                })
                .collect()
        }
        let items = list_node
            .children()
            .enumerate()
            .map(|(index, child)| match child.data.borrow().value {
                NodeValue::Item(item_node_list) => {
                    item_node_to_text(child, index, level, &item_node_list, plain)
                }
                _ => {
                    eprintln!("💔 Unexpected child in List node: {:#?}", child);
                    "💔 Unexpected child in List node".to_string()
                }
            })
            .collect::<Vec<String>>()
            .join("");
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
            _ => {
                eprintln!("💔 Unexpected child in List Item node: {:#?}", child);
                document.push("💔 Unexpected child in List Item node".to_string());
            }
        });
    document.join("")
}
