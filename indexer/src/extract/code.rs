use anyhow::Result;
use tree_sitter::{Node, Parser};

use super::{Extraction, Language, Tags};

pub(super) fn extract(language: Language, extraction: &mut Extraction<'_>) -> Result<()> {
    let grammar = match language {
        Language::JavaScript => tree_sitter_javascript::LANGUAGE,
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
        Language::Tsx => tree_sitter_typescript::LANGUAGE_TSX,
        Language::Markdown => unreachable!("Markdown is parsed separately"),
    };
    let mut parser = Parser::new();
    parser.set_language(&grammar.into())?;
    let source = extraction.source;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("Tree-sitter did not produce a tree"))?;
    let root = tree.root_node();
    let mut cursor = root.walk();
    let header_end = root
        .named_children(&mut cursor)
        .find(|node| !matches!(node.kind(), "comment" | "hash_bang_line"))
        .map_or(source.len(), |node| node.start_byte());
    cursor.reset(root);

    loop {
        let node = cursor.node();
        if node.kind() == "comment" {
            extract_comment(node, header_end, extraction);
        }
        if cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return Ok(());
            }
        }
    }
}

fn extract_comment(node: Node<'_>, header_end: usize, extraction: &mut Extraction<'_>) {
    let source = extraction.source;
    let comment = &source[node.byte_range()];
    let Some(body) = comment.strip_prefix("//").or_else(|| {
        comment.strip_prefix("/*")?.strip_suffix("*/")
    }) else {
        return;
    };
    let tags = Tags::parse(body);
    if node.end_byte() <= header_end {
        for feature in &tags.file_features {
            extraction.add(feature, None, None);
        }
    }
    if tags.start || tags.end {
        extraction.block_marker(node.byte_range(), tags);
    } else if comment.starts_with("/**") && !tags.features.is_empty()
        && let Some(next) = node.next_named_sibling()
        && source[node.end_byte()..next.start_byte()].trim().is_empty()
    {
        extract_symbols(next, &tags.features, extraction);
    }
}

fn extract_symbols(target: Node<'_>, features: &[&str], extraction: &mut Extraction<'_>) {
    let declaration = if matches!(target.kind(), "export_statement" | "ambient_declaration") {
        target.child_by_field_name("declaration")
            .or_else(|| target.child_by_field_name("value"))
            .or_else(|| target.named_child(0))
            .unwrap_or(target)
    } else {
        target
    };
    let mut add = |node: Node<'_>| {
        if let Some(symbol) = symbol_name(node, extraction.source) {
            for feature in features {
                extraction.add(feature, Some(&symbol), Some(target.byte_range()));
            }
        }
    };
    match declaration.kind() {
        "lexical_declaration" | "variable_declaration" => {
            let mut cursor = declaration.walk();
            for variable in declaration.named_children(&mut cursor) {
                if variable.kind() == "variable_declarator"
                    && variable.child_by_field_name("name").is_some_and(|name| {
                        name.kind() == "identifier"
                    })
                {
                    add(variable);
                }
            }
        }
        "function_declaration" | "generator_function_declaration" | "function_signature"
        | "function_expression" | "class_declaration" | "abstract_class_declaration"
        | "interface_declaration" | "type_alias_declaration" | "enum_declaration"
        | "internal_module" | "module" => add(declaration),
        _ if is_member(declaration) => add(declaration),
        _ => {}
    }
}

fn is_member(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "method_definition"
            | "method_signature"
            | "abstract_method_signature"
            | "public_field_definition"
            | "field_definition"
            | "property_signature"
    ) || node.parent().is_some_and(|parent| parent.kind() == "enum_body")
}

fn name_node(node: Node<'_>) -> Option<Node<'_>> {
    node.child_by_field_name("name")
        .or_else(|| node.child_by_field_name("property"))
}

fn symbol_name(node: Node<'_>, source: &str) -> Option<String> {
    let name = name_node(node).or_else(|| {
        // Unassigned enum members are bare name nodes rather than declarations.
        node.parent()
            .filter(|parent| parent.kind() == "enum_body" && node.kind() != "enum_assignment")
            .map(|_| node)
    })?;
    let mut parts = vec![&source[name.byte_range()]];
    if is_member(node) {
        let mut ancestor = node.parent();
        while let Some(owner) = ancestor {
            if matches!(owner.kind(), "statement_block" | "block")
                && !owner.parent().is_some_and(|parent| {
                    matches!(parent.kind(), "internal_module" | "module")
                })
            {
                break;
            }
            if matches!(
                owner.kind(),
                "class"
                    | "class_declaration"
                    | "abstract_class_declaration"
                    | "interface_declaration"
                    | "type_alias_declaration"
                    | "enum_declaration"
                    | "internal_module"
                    | "module"
                    | "property_signature"
            ) && let Some(name) = name_node(owner)
            {
                parts.push(&source[name.byte_range()]);
            }
            ancestor = owner.parent();
        }
    }
    parts.reverse();
    Some(parts.join("."))
}
